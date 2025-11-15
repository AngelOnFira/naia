cfg_if! {
    if #[cfg(feature = "transport_webrtc")] {
        pub mod webrtc;
    } else {}
}
cfg_if! {
    if #[cfg(feature = "transport_udp")] {
        pub mod udp;
    } else {}
}
cfg_if! {
    if #[cfg(feature = "transport_quic")] {
        pub mod quic;
    } else {}
}

mod server_addr;

pub use server_addr::ServerAddr;

pub use inner::{
    IdentityReceiver, IdentityReceiverResult, PacketReceiver, PacketSender, RecvError, SendError,
    Socket, StreamReceiver, StreamSender,
};

mod inner {

    use naia_shared::IdentityToken;
    pub enum IdentityReceiverResult {
        Waiting,
        Success(IdentityToken),
        ErrorResponseCode(u16),
    }

    use super::ServerAddr;

    pub struct SendError;

    pub struct RecvError;

    pub trait Socket {
        fn connect(
            self: Box<Self>,
        ) -> (
            Box<dyn IdentityReceiver>,
            Box<dyn PacketSender>,
            Box<dyn PacketReceiver>,
            Box<dyn StreamSender>,
            Box<dyn StreamReceiver>,
        );
        fn connect_with_auth(
            self: Box<Self>,
            auth_bytes: Vec<u8>,
        ) -> (
            Box<dyn IdentityReceiver>,
            Box<dyn PacketSender>,
            Box<dyn PacketReceiver>,
            Box<dyn StreamSender>,
            Box<dyn StreamReceiver>,
        );
        fn connect_with_auth_headers(
            self: Box<Self>,
            auth_headers: Vec<(String, String)>,
        ) -> (
            Box<dyn IdentityReceiver>,
            Box<dyn PacketSender>,
            Box<dyn PacketReceiver>,
            Box<dyn StreamSender>,
            Box<dyn StreamReceiver>,
        );
        fn connect_with_auth_and_headers(
            self: Box<Self>,
            auth_bytes: Vec<u8>,
            auth_headers: Vec<(String, String)>,
        ) -> (
            Box<dyn IdentityReceiver>,
            Box<dyn PacketSender>,
            Box<dyn PacketReceiver>,
            Box<dyn StreamSender>,
            Box<dyn StreamReceiver>,
        );
    }

    pub trait PacketSender: Send + Sync {
        /// Sends a packet from the Client Socket
        fn send(&self, payload: &[u8]) -> Result<(), SendError>;
        /// Get the Server's Socket address
        fn server_addr(&self) -> ServerAddr;
    }

    pub trait PacketReceiver: PacketReceiverClone + Send + Sync {
        /// Receives a packet from the Client Socket
        fn receive(&mut self) -> Result<Option<&[u8]>, RecvError>;
        /// Get the Server's Socket address
        fn server_addr(&self) -> ServerAddr;
    }

    /// Used to clone Box<dyn PacketReceiver>
    pub trait PacketReceiverClone {
        /// Clone the boxed PacketReceiver
        fn clone_box(&self) -> Box<dyn PacketReceiver>;
    }

    impl<T: 'static + PacketReceiver + Clone> PacketReceiverClone for T {
        fn clone_box(&self) -> Box<dyn PacketReceiver> {
            Box::new(self.clone())
        }
    }

    impl Clone for Box<dyn PacketReceiver> {
        fn clone(&self) -> Box<dyn PacketReceiver> {
            PacketReceiverClone::clone_box(self.as_ref())
        }
    }

    // Stream

    pub trait StreamSender: Send + Sync {
        /// Sends a large message via stream (reliable, ordered, no fragmentation)
        fn send(&self, payload: &[u8]) -> Result<(), SendError>;
    }

    pub trait StreamReceiver: StreamReceiverClone + Send + Sync {
        /// Receives a complete message from a stream
        fn receive(&mut self) -> Result<Option<Vec<u8>>, RecvError>;
    }

    /// Used to clone Box<dyn StreamReceiver>
    pub trait StreamReceiverClone {
        /// Clone the boxed StreamReceiver
        fn clone_box(&self) -> Box<dyn StreamReceiver>;
    }

    impl<T: 'static + StreamReceiver + Clone> StreamReceiverClone for T {
        fn clone_box(&self) -> Box<dyn StreamReceiver> {
            Box::new(self.clone())
        }
    }

    impl Clone for Box<dyn StreamReceiver> {
        fn clone(&self) -> Box<dyn StreamReceiver> {
            StreamReceiverClone::clone_box(self.as_ref())
        }
    }

    // Identity

    pub trait IdentityReceiver: IdentityReceiverClone + Send + Sync {
        ///
        fn receive(&mut self) -> IdentityReceiverResult;
    }

    /// Used to clone Box<dyn IdentityReceiver>
    pub trait IdentityReceiverClone {
        /// Clone the boxed IdentityReceiver
        fn clone_box(&self) -> Box<dyn IdentityReceiver>;
    }

    impl<T: 'static + IdentityReceiver + Clone> IdentityReceiverClone for T {
        fn clone_box(&self) -> Box<dyn IdentityReceiver> {
            Box::new(self.clone())
        }
    }

    impl Clone for Box<dyn IdentityReceiver> {
        fn clone(&self) -> Box<dyn IdentityReceiver> {
            IdentityReceiverClone::clone_box(self.as_ref())
        }
    }
}
