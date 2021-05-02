use crate::do_serialize;
use canadensis_can::{OutOfMemoryError, Transmitter};
use canadensis_core::time::Instant;
use canadensis_core::transfer::{ServiceHeader, Transfer, TransferHeader, TransferKindHeader};
use canadensis_core::{NodeId, Priority, ServiceId, TransferId};
use canadensis_encoding::Serialize;

/// Assembles transfers and manages transfer IDs to send service requests
pub struct Requester<I: Instant> {
    /// The ID of this node
    this_node: NodeId,
    /// The priority of transfers from this transmitter
    priority: Priority,
    /// The timeout for sending transfers
    timeout: I::Duration,
    /// The ID of the next transfer to send, for each destination node
    next_transfer_ids: NextTransferIds,
}

impl<I: Instant> Requester<I> {
    /// Creates a service request transmitter
    ///
    /// this_node: The ID of this node
    ///
    /// priority: The priority to use for messages
    ///
    /// service: The service ID to request
    pub fn new(this_node: NodeId, timeout: I::Duration, priority: Priority) -> Self {
        Requester {
            this_node,
            priority,
            timeout,
            next_transfer_ids: NextTransferIds::new(),
        }
    }

    pub fn send<T>(
        &mut self,
        now: I,
        service: ServiceId,
        payload: &T,
        destination: NodeId,
        transmitter: &mut Transmitter<I>,
    ) -> Result<(), OutOfMemoryError>
    where
        T: Serialize,
    {
        // Part 1: Serialize
        let deadline = self.timeout.clone() + now;
        do_serialize(payload, |payload_bytes| {
            // Part 2: Split into frames and send
            self.send_payload(payload_bytes, service, destination, deadline, transmitter)
        })
    }

    pub fn send_payload(
        &mut self,
        payload: &[u8],
        service: ServiceId,
        destination: NodeId,
        deadline: I,
        transmitter: &mut Transmitter<I>,
    ) -> Result<(), OutOfMemoryError> {
        // Assemble the transfer
        let transfer_id = self.next_transfer_ids.get_and_increment(destination);
        let transfer: Transfer<&[u8], I> = Transfer {
            timestamp: deadline,
            header: TransferHeader {
                source: self.this_node,
                priority: self.priority,
                kind: TransferKindHeader::Request(ServiceHeader {
                    service,
                    destination,
                }),
            },
            transfer_id,
            payload,
        };

        transmitter.push(transfer)
    }
}

/// A map from destination node IDs to transfer IDs of the next transfer
struct NextTransferIds {
    ids: [TransferId; NodeId::MAX.to_u8() as usize],
}

impl NextTransferIds {
    /// Creates a new transfer ID map with the default transfer ID for each node
    pub fn new() -> Self {
        NextTransferIds {
            ids: [TransferId::default(); NodeId::MAX.to_u8() as usize],
        }
    }
    /// Returns the next transfer ID for the provided node, and increments the stored transfer
    /// ID
    pub fn get_and_increment(&mut self, destination: NodeId) -> TransferId {
        let entry = &mut self.ids[usize::from(destination)];
        let current = *entry;
        *entry = entry.increment();
        current
    }
}
