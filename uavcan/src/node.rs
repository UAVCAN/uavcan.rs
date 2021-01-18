//! The Node struct is a conveniance wrapper around the Transport and SessionManager
//! implementations. Currently it just handles ingesting and transmitting data, although
//! it might make sense in the future to split these up into seperate concepts. Currently
//! the only coupling between TX and RX is the node ID, which can be cheaply replicated.
//! It might be prudent to split out Messages and Services, into seperate concepts (e.g.
//! Publisher, Requester, Responder, and Subscriber, a la canadensis, but I'll need to
//! play around with those concepts before I commit to anything)

use core::marker::PhantomData;

use crate::types::*;
use crate::RxError;
use crate::transfer::Transfer;
use crate::transport::Transport;
use crate::session::SessionManager;

/// Node implementation. Generic across session managers and transport types.
#[derive(Copy, Clone, Debug)]
pub struct Node<S: SessionManager, T: Transport> {
    // TODO this is transport level type
    id: Option<NodeId>,

    /// Session manager. Made public so it could be managed by implementation.
    ///
    /// Instead of being public, could be placed behind a `with_session_manager` fn
    /// which took a closure. I can't decide which API is better.
    pub sessions: S,

    /// Transport type
    transport: PhantomData<T>,
}

impl<S: SessionManager, T: Transport> Node<S, T> {
    // TODO needs to accept a SessionManager
    pub fn new(id: Option<NodeId>, session_manager: S) -> Self {
        Self {
            id,
            sessions: session_manager,
            transport: PhantomData,
        }
    }

    // Convenience function to access session manager inside of a closure.
    // I was going to use this because I was thinking I needed a closure
    // to access the session manager safely, but that isn't really the case.
    //
    // It still has potential to be useful (i.e. if you're using this with
    // an unsafe storage mechanism, the below form will prevent you from
    // taking references of the session manager), but idk if it actually is.
    //fn with_session_manager<R>(&mut self, f: fn(&mut T) -> R) -> R {
    //    f(&mut self.sessions)
    //}

    /// Attempts to receive frame. Returns error when frame is invalid, Some(Transfer) at the end of
    /// a transfer, and None if we haven't finished the transfer.
    pub fn try_receive_frame<'a>(
        &mut self,
        frame: T::Frame,
    ) -> Result<Option<Transfer>, RxError> {
        // TODO check extended ID mask etc.
        let frame = T::rx_process_frame(&self.id, &frame)?;

        if let Some(frame) = frame {
            match self.sessions.ingest(frame) {
                Ok(frame) => Ok(frame),
                Err(err) => Err(RxError::SessionError(err)),
            }
        } else {
            Ok(None)
        }
    }

    // Create a series of frames to transmit.
    // I think there could be 3 versions of this:
    // 1. Returns a collection of frames to transmit.
    // 2. Pushes frame onto queue, similar to libcanard.
    // 3. Returns an iterator into a series of frames.
    //
    // 1 and 3 provide the user with more options but also make it harder
    // to implement for the user.
    //pub fn transmit(&self, transfer: &Transfer) -> T::FrameIter {
    //    T::transmit(transfer)
    //}
}