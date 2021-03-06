//! This module contains everything related to the transfer protocol that will be used to transmit the uavcan frame
//!
//! The only transfer protocol that is currently supported by the uavcan protocol is CAN2.0B.

use lib::core::convert::From;
use lib::core::cmp::Ordering;

use embedded_types;

pub use embedded_types::io::Error as IOError;

/// `TransferInterface` is an interface to a hardware unit which can communicate over a CAN like transfer protocol
///
/// It's associated with a `TransferFrame` and must be able to receive and transmit this type of frames.
/// The interface must also do ordering of incoming frames after priority defined by the transfer frame ID to avoid priority inversion,
/// while making sure that transfer frames with the same ID is transmitted in the same order as they was added in the transmit buffer.
///
/// Receiving frames must be returned in the same order they were received by the interface.
pub trait TransferInterface {
    /// The TransferFrame associated with this interface.
    type Frame: TransferFrame;

    /// Put a `TransferFrame` in the transfer buffer (or transmit it on the bus) or return `Err(IOError::BufferExhausted)` if buffer is full.
    ///
    /// To avoid priority inversion the new frame needs to be prioritized inside the interface as it would on the bus.
    /// When reprioritizing the `TransferInterface` must for equal ID frames respect the order they were attempted transmitted in.
    fn transmit(&self, frame: &Self::Frame) -> Result<(), IOError>;
    
    /// Receive a frame, removing to from the receive buffer.
    /// if there are no frames in the receive buffer this function will return `None`
    fn receive(&self) -> Option<Self::Frame>;
}

/// `TransferFrame` is a CAN like frame that can be sent over a network
///
/// For a frame to work it need to have a 28 bit ID, and a payload of
/// at least 4 bytes. Guarantee that frames are delivered in order
/// and correctness check is needed as well.
///
/// The uavcan protocol defines how this works with a CAN2.0B frame
pub trait TransferFrame {
    /// Maximum data length the transfer protocol supports.
    const MAX_DATA_LENGTH: usize;

    /// Create a new TransferFrame with id: id, and length 0.
    /// Data length can be changed with `set_data_length(&self)`.
    /// Data can be changed with `data_as_mut(&mut self)`.
    fn new(id: TransferFrameID) -> Self;

    /// Returns the 28 bit ID of this TransferFrame.
    ///
    /// When deciding which frame that will be transmitted,
    /// the ID is used to prioritze (lower ID means higher priority)
    fn id(&self) -> TransferFrameID;

    /// Returns a slice with the data in this TransferFrame
    ///
    /// Length can be found by checking the length
    /// of this slice `self.data().len()`
    fn data(&self) -> &[u8];

    /// Returns a mutable slice with the data in this TransferFrame
    /// use this method to set/change the data inside this TransferFrame
    fn data_as_mut(&mut self) -> &mut [u8];

    /// Set the data length of this TransferFrame
    ///
    /// ## Panics
    /// `set_data_lengt(&mut self, length: usize)` should panic if `length > T::MAX_DATA_LENGTH`
    fn set_data_length(&mut self, length: usize);
    
    /// Returns the tail byte of the TransferFrame assuming the current length
    ///
    /// ## Panics
    /// panics if `self.data().len() == 0` as no tail_byte exists
    fn tail_byte(&self) -> TailByte {
        TailByte::from(
            *self.data()
                .last()
                .expect("Can't return tail byte of frame with 0 data bytes"),
        )
    }

    /// Checks the tail byte if this frame is a start frame and return the result
    fn is_start_frame(&self) -> bool {
        self.tail_byte().start_of_transfer()
    }
    
    /// Checks the tail byte if this frame is an end frame and return the result
    fn is_end_frame(&self) -> bool {
        self.tail_byte().end_of_transfer()
    }
    
    /// Checks the tail byte if this is both a start frame and an end frame and return the result
    fn is_single_frame(&self) -> bool {
        self.is_end_frame() && self.is_start_frame()
    }

    /// Returns the full ID of the frame (both Frame ID and transfer ID)
    ///
    /// ## Panics
    /// panics if `self.data().len() == 0` as no tail_byte exists
    fn full_id(&self) -> FullTransferID {
        FullTransferID {
            frame_id: self.id(),
            transfer_id: self.tail_byte().transfer_id(),
        }
    }
}


/// Cotains both the `TransferFrameID` and `TransferID` to uniquely distinguish a transfer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct FullTransferID {
    pub frame_id: TransferFrameID,
    pub transfer_id: TransferID,
}

impl FullTransferID {
    /// Deasserts bits based on the asserted bits of `mask`
    pub fn mask(self, mask: Self) -> Self {
        FullTransferID {
            frame_id: self.frame_id.mask(mask.frame_id),
            transfer_id: self.transfer_id.mask(mask.transfer_id),
        }
    }
}

/// The 29-bit ID of a `TransferFrame`
///
/// Frames that will win arbitration can be found by the help of ordering.
/// If `frame1 < frame2`, then `frame1` will win arbitration over `frame2`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct TransferFrameID(u32);

impl TransferFrameID {
    /// Constructs a new `TransferFrameID`
    /// ## Panic
    /// Panics if `value` is something not representable with 29-bits
    pub fn new(value: u32) -> TransferFrameID {
        assert_eq!(value & !0x1fff_ffff, 0);
        TransferFrameID(value)
    }
    
    /// Deasserts bits based on the asserted bits of `mask`
    pub fn mask(self, mask: Self) -> Self {
        let TransferFrameID(mut value) = self;
        value &= u32::from(mask);
        TransferFrameID(value)        
    }
}

impl From<TransferFrameID> for u32 {
    fn from(id: TransferFrameID) -> u32 {
        let TransferFrameID(value) = id;
        value
    }
}

/// A filter for `TransferFrameID`
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct TransferFrameIDFilter{
    value: u32,
    mask: u32,
}

impl TransferFrameIDFilter {
    pub fn new(value: u32, mask: u32) -> Self {
        TransferFrameIDFilter{
            mask: mask,
            value: value,
        }
    }

    pub fn is_match(&self, value: TransferFrameID) -> bool {
        self.mask & u32::from(value) == self.mask & self.value
    }
}

/// The 5-bit ID used to distinguish consecutive transfers
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct TransferID(u8);

impl TransferID {
    /// Constructs a new `TransferID`
    /// ## Panic
    /// Panics if `value` is something not representable with 5-bits
    pub fn new(value: u8) -> TransferID {
        assert_eq!(value & !0x1f, 0);
        TransferID(value)
    }
    
    /// Deasserts bits based on the asserted bits of `mask`
    pub fn mask(self, mask: Self) -> Self {
        let TransferID(mut value) = self;
        value &= u8::from(mask);
        TransferID(value)        
    }
}

impl From<TransferID> for u8 {
    fn from(tid: TransferID) -> u8 {
        let TransferID(value) = tid;
        value
    }
}

/// A Wrapper that can be used for sorting after priority
///
/// PartialEq, Eq, PartialOrd and Ord is implemented but only cares about priority.
///
/// # Examples
/// ## Usable with `TransferFrameID`
/// ```
/// use uavcan::transfer::TransferFrameID;
/// use uavcan::transfer::Priority;
///
/// assert!(Priority(TransferFrameID::new(0)) > Priority(TransferFrameID::new(1)))
///
/// ```
///
/// ## Usable with anything that implements `TransferFrame`
/// ```
/// use uavcan::transfer::Priority;
/// use uavcan::transfer::TransferFrameID;
/// use uavcan::transfer::TransferFrame;
///
/// #[derive(Debug, PartialEq)]
/// pub struct CanFrame {
///     pub id: TransferFrameID,
///     pub dlc: usize,
///     pub data: [u8; 8],
/// }
///
/// impl TransferFrame for CanFrame {
///     const MAX_DATA_LENGTH: usize = 8;
///
///     fn new(id: TransferFrameID) -> CanFrame {
///         CanFrame{id: id, dlc: 0, data: [0; 8]}
///     }
///
///     fn set_data_length(&mut self, length: usize) {
///         assert!(length <= 8);
///         self.dlc = length;
///     }
///
///     fn data(&self) -> &[u8] {
///         &self.data[0..self.dlc]
///     }
///
///     fn data_as_mut(&mut self) -> &mut[u8] {
///         &mut self.data[0..self.dlc]
///     }
///
///     fn id(&self) -> TransferFrameID {
///         self.id
///     }
/// }
///
/// assert!(Priority(CanFrame::new(TransferFrameID::new(0))) > Priority(CanFrame::new(TransferFrameID::new(1))));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Priority<T>(pub T);



impl Eq for Priority<TransferFrameID> {}

impl PartialEq for Priority<TransferFrameID> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Ord for Priority<TransferFrameID> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0).reverse()
    }
}

impl PartialOrd for Priority<TransferFrameID> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}



impl<F: TransferFrame> Eq for Priority<F> {}


impl<F: TransferFrame> PartialEq for Priority<F> {
    fn eq(&self, other: &Self) -> bool {
        self.0.id().eq(&other.0.id())
    }
}

impl<F: TransferFrame> Ord for Priority<F> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.id().cmp(&other.0.id()).reverse()
    }
}

impl<F: TransferFrame> PartialOrd for Priority<F> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}


/// The last byte of the transfer frame data field, which contains auxiliary transport layer fields.
///
/// | SOT | EOT | Toggle | TID | TID | TID | TID | TID |
/// |-----|-----|--------|-----|-----|-----|-----|-----|
/// | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
///
/// ### Start of transfer (SOT)
/// For single-frame transfers, the value of this field is always 1.
///
/// For multi-frame transfers, the value of this field is 1 if the current frame is the first frame of the transfer, and 0 otherwise.
///
/// ### End of transfer (EOT)
/// For single-frame transfers, the value of this field is always 1.
///
/// For multi-frame transfers, the value of this field is 1 if the current frame is the last frame of the transfer, and 0 otherwise.
///
/// ### Toggle
/// For single-frame transfers, the value of this field is always 0.
///
/// For multi-frame transfers, this field contains the value of the toggle bit. As specified above this will alternate value between frames, starting at 0 for the first frame.
///
/// ### Transfer ID (TID)
/// This field contains the transfer ID value of the current transfer (for all types of transfers).
///
/// The value is 5 bits wide, therefore the allowed values range from 0 to 31, inclusively.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TailByte(u8);

impl TailByte {
    pub fn new(
        start_of_transfer: bool,
        end_of_transfer: bool,
        toggle: bool,
        transfer_id: TransferID,
    ) -> Self {
        TailByte(
            ((start_of_transfer as u8) << 7) |
            ((end_of_transfer as u8) << 6) |
            ((toggle as u8) << 5) |
            (u8::from(transfer_id))
        )
    }

    /// Checks if the SOT bit is asserted
    pub fn start_of_transfer(&self) -> bool {
        let TailByte(value) = *self;
        value & (1 << 7) != 0
    }

    /// Checks if the EOT bit is asserted
    pub fn end_of_transfer(&self) -> bool {
        let TailByte(value) = *self;
        value & (1 << 6) != 0
    }
    
    /// Checks if the toggle bit is asserted
    pub fn toggle(&self) -> bool {
        let TailByte(value) = *self;
        value & (1 << 5) != 0
    }
    
    /// Returns the `TransferID`
    pub fn transfer_id(&self) -> TransferID {
        let TailByte(value) = *self;
        TransferID::new(value & 0x1f)
    }
}


impl From<TailByte> for u8 {
    fn from(tb: TailByte) -> u8 {
        let TailByte(value) = tb;
        value
    }
}

impl From<u8> for TailByte {
    fn from(value: u8) -> TailByte {
        TailByte(value)
    }
}






impl From<TransferFrameID> for embedded_types::can::ExtendedID {
    fn from(id: TransferFrameID) -> Self {
        embedded_types::can::ExtendedID::new(u32::from(id))
    }
}

impl From<TransferFrameID> for embedded_types::can::ID {
    fn from(id: TransferFrameID) -> Self {
        embedded_types::can::ID::ExtendedID(embedded_types::can::ExtendedID::from(id))
    }
}

impl From<embedded_types::can::ExtendedID> for TransferFrameID {
    fn from(id: embedded_types::can::ExtendedID) -> Self {
        TransferFrameID::new(u32::from(id))
    }
}

impl TransferFrame for embedded_types::can::ExtendedDataFrame {
    const MAX_DATA_LENGTH: usize = 8;

    fn new(id: TransferFrameID) -> Self {
        embedded_types::can::ExtendedDataFrame::new(id.into())
    }
    fn set_data_length(&mut self, length: usize) {
        assert!(
            length <= Self::MAX_DATA_LENGTH,
            "ExtendedDataFrame::set_data_length() needs the length to be less than 8"
        );
        self.set_data_length(length);
    }
    fn data(&self) -> &[u8] {
        &self.data()
    }
    fn data_as_mut(&mut self) -> &mut [u8] {
        self.data_as_mut()
    }
    fn id(&self) -> TransferFrameID {
        self.id().into()
    }
}
