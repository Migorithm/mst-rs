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
    pub fn new(max_count: u64) -> Self {
        Self {
            hash: [0; 32],
            children: vec![],
            max_count,
        }
    }

    pub fn hash(&self) -> &[u8; 32] {
        &self.hash
    }

    fn xor_hash(&mut self, child_hash: [u8; 32]) {
        for i in 0..32 {
            self.hash[i] ^= child_hash[i]
        }
    }

    pub fn insert(&mut self, key: String, value: String) {
        // Create Leaf
        let mut hasher = sha2::Sha256::new();
        hasher.update(value.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();
        let leaf = Leaf { key, hash };

        // In case of first insertion
        if self.children.is_empty() {
            // Case: Empty Tree
            let mut new_node = InternalNode::new();
            new_node.add_leaf(leaf);
            self.xor_hash(new_node.hash); // Update root hash
            self.children.push(new_node);
            return;
        }

        // Locate Correct Child Node
        let child_index = self
            .children
            .partition_point(|node| &node.max_key < &leaf.key);

        let target_index = if child_index < self.children.len() {
            // Case: Key fits in an existing range
            child_index
        } else {
            // Case: Key is larger than all ranges, child_index is equal to lenth of vector in this case.
            child_index - 1
        };

        // Cancel out old hash
        self.xor_hash(self.children[target_index].hash);

        self.children[target_index].add_leaf(leaf);

        // Reapply hash of the children
        self.xor_hash(self.children[target_index].hash);

        // 5. Handle Node Splitting
        if self.children[target_index].children.len() as u64 > self.max_count {
            // TODO: Split the node
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

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn addleaf_sort_correctly() {}

    #[test]
    fn addleaf_on_same_leaf_correctly_modify_hash() {}

    #[test]
    fn addleaf_creates_deterministic_hash() {}

    #[test]
    fn insert_creates_nodes_correctly() {
        let mut root = Root::new(10);
        root.insert("key1".to_string(), "value1".to_string());

        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].children.len(), 1);
        assert_eq!(root.children[0].children[0].key, "key1");
        assert_eq!(root.children[0].max_key, "key1");

        root.insert("key3".to_string(), "value3".to_string());
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].children.len(), 2);
        assert_eq!(root.children[0].children[1].key, "key3");
        assert_eq!(root.children[0].max_key, "key3"); // max_key is updated

        root.insert("key2".to_string(), "value2".to_string());
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].children.len(), 3);
        assert_eq!(root.children[0].children[0].key, "key1");
        assert_eq!(root.children[0].children[1].key, "key2");
        assert_eq!(root.children[0].children[2].key, "key3");
        assert_eq!(root.children[0].max_key, "key3"); // max_key is still key3
    }

    #[test]
    fn insert_updates_root_hash() {
        let mut root = Root::new(10);
        let initial_hash = root.hash().clone();

        root.insert("key1".to_string(), "value1".to_string());
        let hash_after_1 = root.hash().clone();
        assert_ne!(initial_hash, hash_after_1);

        root.insert("key2".to_string(), "value2".to_string());
        let hash_after_2 = root.hash().clone();
        assert_ne!(hash_after_1, hash_after_2);
    }
}
