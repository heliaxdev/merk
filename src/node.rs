use std::cmp::max;
use std::fmt;

use blake2_rfc::blake2b::Blake2b;
use byteorder::{BigEndian, ByteOrder};

use crate::error::*;

const HASH_LENGTH: usize = 20;
const NULL_HASH: Hash = [0; HASH_LENGTH];

type Hash = [u8; HASH_LENGTH];

/// Represents a reference to another tree node.
///
/// Note that the referenced node is not necessarily
/// loaded in memory, but can be fetched from a backing
/// database by its key.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Link {
    pub key: Vec<u8>,
    pub hash: Hash,
    pub height: u8,
}

impl fmt::Debug for Link {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "(key={}, hash={}, height={})",
            String::from_utf8(self.key.to_vec()).unwrap(),
            hex::encode(&self.hash[0..3]),
            self.height
        )
    }
}

/// Represents a tree node and its associated key/value pair.
#[derive(Serialize, Deserialize, Clone)]
pub struct Node {
    // don't serialize key since it's implied from the db
    #[serde(skip)]
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub kv_hash: Hash,
    pub parent_key: Option<Vec<u8>>,
    pub left: Option<Link>,
    pub right: Option<Link>,
}

/// Replaces the value of a `Vec<T>` by cloning into it,
/// possibly not needing to allocate.
fn set_vec<T: Clone>(dest: &mut Vec<T>, src: &[T]) {
    dest.clear();
    dest.extend_from_slice(src);
}

///
impl Node {
    /// Creates a new node from a key and value.
    pub fn new(key: &[u8], value: &[u8]) -> Node {
        let mut node = Node {
            key: key.to_vec(),
            value: value.to_vec(),
            kv_hash: Default::default(),
            parent_key: None,
            left: None,
            right: None,
        };
        node.update_kv_hash();
        node
    }

    pub fn decode(key: &[u8], bytes: &[u8]) -> Result<Node> {
        let node = bincode::deserialize(bytes).map(|mut node: Node| {
            set_vec(&mut node.key, key);
            node
        })?;
        Ok(node)
    }

    pub fn update_kv_hash(&mut self) {
        // TODO: make generic to allow other hashers
        let mut hasher = Blake2b::new(HASH_LENGTH);

        hasher.update(&[self.key.len() as u8]);
        hasher.update(&self.key);

        let mut val_length = [0; 2];
        BigEndian::write_u16(&mut val_length, self.value.len() as u16);
        hasher.update(&val_length);

        hasher.update(&self.value);

        let res = hasher.finalize();
        self.kv_hash.copy_from_slice(res.as_bytes());
    }

    pub fn hash(&self) -> Hash {
        // TODO: make generic to allow other hashers
        let mut hasher = Blake2b::new(HASH_LENGTH);
        hasher.update(&self.kv_hash);
        hasher.update(match &self.left {
            Some(left) => &(left.hash),
            None => &NULL_HASH,
        });
        hasher.update(match &self.right {
            Some(right) => &(right.hash),
            None => &NULL_HASH,
        });
        let res = hasher.finalize();
        let mut hash: Hash = Default::default();
        hash.copy_from_slice(res.as_bytes());
        hash
    }

    pub fn child_link(&self, left: bool) -> Option<Link> {
        if left {
            self.left.clone()
        } else {
            self.right.clone()
        }
    }

    pub fn child_height(&self, left: bool) -> u8 {
        let link = self.child_link(left);
        match link {
            Some(link) => link.height,
            None => 0,
        }
    }

    pub fn height(&self) -> u8 {
        max(self.child_height(true), self.child_height(false)) + 1
    }

    pub fn balance_factor(&self) -> i8 {
        self.child_height(false) as i8 - self.child_height(true) as i8
    }

    pub fn as_link(&self) -> Link {
        Link {
            key: self.key.to_vec(),
            hash: self.hash(),
            height: self.height(),
        }
    }

    pub fn set_child(&mut self, left: bool, link: Option<Link>) {
        if left {
            self.left = link;
        } else {
            self.right = link;
        }
    }

    pub fn set_parent(&mut self, parent_key: Option<Vec<u8>>) {
        self.parent_key = parent_key;
    }

    pub fn set_value(&mut self, value: &[u8]) {
        set_vec(&mut self.value, value);
        self.update_kv_hash();
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        let bytes = bincode::serialize(&self)?;
        Ok(bytes)
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "({:?}: {:?}, hash={})",
            &self.key, //String::from_utf8(self.key.to_vec()).unwrap(),
            &self.key, //String::from_utf8(self.value.to_vec()).unwrap(),
            hex::encode(&self.hash()[..])
        )
    }
}