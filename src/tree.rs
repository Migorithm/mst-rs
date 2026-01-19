use sha2::Digest;

struct Root<K: Ord + Clone + Default> {
    hash: [u8; 32],
    children: Vec<InternalNode<K>>,
    max_count: u64,
}

struct InternalNode<K: Ord + Clone + Default> {
    hash: [u8; 32],
    children: Vec<Leaf<K>>,
    max_key: K,
}

impl<K: Ord + Clone + Default> InternalNode<K> {
    fn new() -> Self {
        Self {
            hash: [0; 32],
            children: vec![],
            max_key: K::default(),
        }
    }

    fn add_leaf(&mut self, leaf: Leaf<K>) {
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

    fn recalculate(&mut self) {
        self.hash = [0; 32];
        if let Some(last) = self.children.last() {
            self.max_key = last.key.clone();
        } else {
            self.max_key = K::default();
        }

        for hash in self.children.iter().map(|l| l.hash).collect::<Vec<_>>() {
            self.xor_hash(hash);
        }
    }

    fn split(&mut self) -> InternalNode<K> {
        let mid = self.children.len() / 2;
        let other_leaves = self.children.split_off(mid);

        let mut new_node = InternalNode::new();
        new_node.children = other_leaves;

        // Recalculate hashes and max_keys for both nodes
        self.recalculate();
        new_node.recalculate();

        new_node
    }
}

impl<K: Ord + Clone + Default> Root<K> {
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

    pub fn insert(&mut self, key: K, value: String) {
        // Create Leaf
        let mut hasher = sha2::Sha256::new();
        hasher.update(value.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();
        let leaf = Leaf { key, hash };

        // Handle Insertion and Edge Cases
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
            child_index
        } else {
            child_index - 1
        };

        // Cancel out previous hash
        let old_hash = self.children[target_index].hash;
        self.xor_hash(old_hash);

        self.children[target_index].add_leaf(leaf);

        if self.children[target_index].children.len() as u64 > self.max_count {
            // Node is over capacity, split it.
            let new_sibling = self.children[target_index].split();

            self.xor_hash(self.children[target_index].hash);
            self.xor_hash(new_sibling.hash);

            // Add the new sibling to the root's children.
            self.children.insert(target_index + 1, new_sibling);
        } else {
            // No split, just XOR in the new hash.
            self.xor_hash(self.children[target_index].hash);
        }
    }
}

struct Leaf<K: Ord + Clone> {
    key: K,
    hash: [u8; 32],
}

impl<K: Ord + Clone> PartialEq for Leaf<K> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<K: Ord + Clone> Eq for Leaf<K> {}

impl<K: Ord + Clone> PartialOrd for Leaf<K> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.key.partial_cmp(&other.key)
    }
}
impl<K: Ord + Clone> Ord for Leaf<K> {
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

    #[test]
    fn insert_with_split() {
        let mut root = Root::new(2); // max_count = 2

        root.insert("key1".to_string(), "value1".to_string());
        root.insert("key2".to_string(), "value2".to_string());

        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].children.len(), 2);

        // This should trigger a split
        root.insert("key3".to_string(), "value3".to_string());

        assert_eq!(root.children.len(), 2); // Now we have two internal nodes

        // The leaves before split are [key1, key2, key3]. mid = 3 / 2 = 1.
        // Original node (at index 0) is left with 1 element.
        assert_eq!(root.children[0].children.len(), 1);
        assert_eq!(root.children[0].children[0].key, "key1");
        assert_eq!(root.children[0].max_key, "key1");

        // New node (at index 1) gets the other 2.
        assert_eq!(root.children[1].children.len(), 2);
        assert_eq!(root.children[1].children[0].key, "key2");
        assert_eq!(root.children[1].children[1].key, "key3");
        assert_eq!(root.children[1].max_key, "key3");

        // Also check root hash is non-zero and has some value.
        let zero_hash = [0; 32];
        assert_ne!(root.hash(), &zero_hash);
    }
}
