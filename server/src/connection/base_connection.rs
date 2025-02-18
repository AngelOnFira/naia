use naia_shared::{BaseConnection, ConnectionConfig};

use crate::user::UserKey;

pub struct ServerBaseConnection {
    pub user_key: UserKey,
    pub base: BaseConnection,
    pub manual_disconnect: bool,
}

impl ServerBaseConnection {
    pub fn new(connection_config: &ConnectionConfig, user_key: &UserKey) -> Self {
        Self {
            user_key: *user_key,
            base: BaseConnection::new(connection_config),
            manual_disconnect: false,
        }
    }
}
