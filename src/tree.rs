use sha2::Digest;

type MaxKey = String;

struct Root {
    hash: [u8; 32],
    children: Vec<InternalNode>,
    max_count: u64,
}

struct InternalNode {
    hash: [u8; 32],
    children: Vec<Leaf>,
    max_key: MaxKey,
}

impl InternalNode {
    fn new() -> Self {
        Self {
            hash: [0; 32],
            children: vec![],
            max_key: "".to_string(),
        }
    }

    fn add_leaf(&mut self, leaf: Leaf) {
        let leaf_hash = leaf.hash;

        match self.children.binary_search_by(|l| l.key.cmp(&leaf.key)) {
            Ok(index) => {
                let old_hash = self.children[index].hash;
                // ! cancel out previous hash by XORing(because X ^ X = 0)
                self.xor_hash(old_hash);

                self.children[index] = leaf;
                self.xor_hash(leaf_hash);
            }
            Err(index) => {
                self.xor_hash(leaf_hash);
                self.children.insert(index, leaf);
            }
        }

        if let Some(last_leaf) = self.children.last() {
            self.max_key = last_leaf.key.clone();
        }
    }

    fn xor_hash(&mut self, leaf_hash: [u8; 32]) {
        for i in 0..32 {
            self.hash[i] ^= leaf_hash[i]
        }
    }
}

impl Root {
    pub fn hash(&self) -> &[u8; 32] {
        &self.hash
    }

    pub fn insert(&mut self, key: String, value: String) {
        // find children which can host the key
        let mut hasher = sha2::Sha256::new();
        hasher.update(value);
        let hash = hasher.finalize();

        match self.children.binary_search_by_key(&&key, |a| &a.max_key) {
            Ok(found) => todo!(),
            Err(not_found) => {
                match (
                    self.children.get(not_found.saturating_sub(1)),
                    self.children.get(not_found),
                ) {
                    (None, None) => {
                        let mut int_node = InternalNode::new();
                        int_node.add_leaf(Leaf {
                            key,
                            hash: hash.into(),
                        });
                    }
                    (None, Some(_)) => todo!(),
                    (Some(_), None) => todo!(),
                    (Some(_), Some(_)) => todo!(),
                }
            }
        }
    }
}

struct Leaf {
    key: String,
    hash: [u8; 32],
}

impl PartialEq for Leaf {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for Leaf {}

impl PartialOrd for Leaf {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.key.partial_cmp(&other.key)
    }
}
impl Ord for Leaf {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}
