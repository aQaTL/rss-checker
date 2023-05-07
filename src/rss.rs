use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

pub struct Entries {
	pub entries: HashMap<String, String>,
}

impl Entries {
	pub fn new() -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(Entries {
			entries: HashMap::new(),
		}))
	}
}
