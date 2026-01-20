use sha2::Digest;
use std::cmp::Ordering;

// The public interface to the tree
pub struct MerkleSearchTree<K> {
    root: Node<K>,
    max_children: usize,
}

// The internal and leaf nodes of the tree

enum Node<K> {
    Internal {
        hash: [u8; 32],
        children: Vec<Node<K>>,
        max_key: K,
    },
    Leaf {
        key: K,
        hash: [u8; 32],
    },
}

impl<K: Default> Default for Node<K> {
    fn default() -> Self {
        Node::Internal {
            hash: [0; 32],
            children: vec![],
            max_key: K::default(),
        }
    }
}

impl<K: Ord + Clone + Default> MerkleSearchTree<K> {
    pub fn new(max_children: usize) -> Self {
        MerkleSearchTree {
            root: Node::default(),
            max_children,
        }
    }

    pub fn insert(&mut self, key: K, value: String) {
        let mut hasher = sha2::Sha256::new();
        hasher.update(value.as_bytes());
        let hash = hasher.finalize().into();

        let leaf = Node::Leaf { key, hash };

        if let Some(new_sibling) = self.root.insert(leaf, self.max_children) {
            // The root split, so we need to create a new root.
            let old_root = std::mem::replace(&mut self.root, Node::default());

            let mut new_root = Node::Internal {
                hash: [0; 32],
                children: vec![old_root, new_sibling],
                max_key: K::default(), // Will be set by recalculate
            };
            new_root.recalculate();
            self.root = new_root;
        }
    }

    pub fn hash(&self) -> &[u8; 32] {
        self.root.hash()
    }
}

impl<K: Ord + Clone + Default> Node<K> {
    fn key(&self) -> &K {
        match self {
            Node::Internal { max_key, .. } => max_key,
            Node::Leaf { key, .. } => key,
        }
    }

    fn hash(&self) -> &[u8; 32] {
        match self {
            Node::Internal { hash, .. } => hash,
            Node::Leaf { hash, .. } => hash,
        }
    }

    fn is_internal(&self) -> bool {
        matches!(self, Node::Internal { .. })
    }

    fn recalculate(&mut self) {
        if let Node::Internal {
            children,
            hash,
            max_key,
        } = self
        {
            *hash = [0; 32];
            if let Some(last_child) = children.last() {
                *max_key = last_child.key().clone();
                for child in children {
                    for i in 0..32 {
                        hash[i] ^= child.hash()[i];
                    }
                }
            } else {
                *max_key = K::default();
            }
        }
    }

    // Inserts a new node into the subtree.
    // Returns a new sibling if the current node splits.
    fn insert(&mut self, new_node: Node<K>, max_children: usize) -> Option<Node<K>> {
        // This method is only callable on Node::Internal
        let children = match self {
            Node::Internal { children, .. } => children,
            Node::Leaf { .. } => panic!("Cannot insert into a leaf node."),
        };

        // Decide whether to descend further or insert at this level.
        // descend if our children are also Internal nodes.
        // insert here if our children are Leaves (or if we have no children yet).
        let are_children_leaves = children.is_empty() || !children[0].is_internal();

        if are_children_leaves {
            //  Base Case: children are leaves. Handle insert/upsert.
            let new_node_key = new_node.key();
            match children.binary_search_by_key(&new_node_key, |child| child.key()) {
                Ok(index) => {
                    // Key exists. Replace the old leaf.
                    children[index] = new_node;
                }
                Err(index) => {
                    // Key not found. Insert the new leaf.
                    children.insert(index, new_node);
                }
            }
        } else {
            // ! Recursive case - children are Internal nodes
            // Find which child to descend into.
            let child_index = children.partition_point(|child| child.key() < new_node.key());

            // Descend and get a potential new sibling from the child if it splits.
            let new_sibling_from_child = children[child_index].insert(new_node, max_children);

            // If the child split, add its new sibling to our children list.
            if let Some(sibling) = new_sibling_from_child {
                let sibling_key = sibling.key();
                let insert_pos = children.partition_point(|c| c.key() < sibling_key);
                children.insert(insert_pos, sibling);
            }
        }

        // After insertion, check if it needs to split itself.
        let my_new_sibling = if children.len() > max_children {
            let mid = children.len() / 2;
            let sibling_children = children.split_off(mid);
            let mut new_sibling = Node::Internal {
                hash: [0; 32],
                children: sibling_children,
                max_key: K::default(), // will be recalculated
            };
            new_sibling.recalculate();
            Some(new_sibling)
        } else {
            None
        };

        // Finally, always recalculate our own hash and max_key before returning.
        self.recalculate();

        my_new_sibling
    }
}

// These are needed for sorting and comparing
impl<K: Ord + Clone + Default> PartialEq for Node<K> {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}
impl<K: Ord + Clone + Default> Eq for Node<K> {}

impl<K: Ord + Clone + Default> PartialOrd for Node<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.key().partial_cmp(other.key())
    }
}
impl<K: Ord + Clone + Default> Ord for Node<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(other.key())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_simple_insert() {
        // insert the first three leaves:
        // 1. insert("key1"): The root has 1 child: [Leaf("key1")]. This is less than 10, so no split.
        // 2. insert("key3"): The root has 2 children: [Leaf("key1"), Leaf("key3")]. This is less than 10, so no split.
        // 3. insert("key2"): The root has 3 children: [Leaf("key1"), Leaf("key2"), Leaf("key3")]. This is still less than 10, so no split.

        let mut tree = MerkleSearchTree::<String>::new(10);
        tree.insert("key1".to_string(), "value1".to_string());
        tree.insert("key3".to_string(), "value3".to_string());
        tree.insert("key2".to_string(), "value2".to_string());

        if let Node::Internal { children, .. } = &tree.root {
            assert_eq!(children.len(), 3);
            assert_eq!(children[0].key(), "key1");
            assert_eq!(children[1].key(), "key2");
            assert_eq!(children[2].key(), "key3");
        } else {
            panic!("Root should be an internal node");
        }
    }

    #[test]
    fn test_cascading_split() {
        let mut tree = MerkleSearchTree::<String>::new(2);
        // These first three inserts will cause a root split (height: 2 -> 3)
        tree.insert("10".to_string(), "v1".to_string());
        tree.insert("20".to_string(), "v2".to_string());
        tree.insert("30".to_string(), "v3".to_string());

        // This does not cause a split.
        tree.insert("05".to_string(), "v4".to_string());

        // This insert causes a split in a child node, which propagates up
        // and causes the root to split again (height: 3 -> 4)
        tree.insert("15".to_string(), "v5".to_string());

        // Verify the final state of the tree (height 4)
        if let Node::Internal { children, .. } = &tree.root {
            // After the second root split, the top root has 2 children
            assert_eq!(children.len(), 2);

            // Inspect the left subtree
            if let Node::Internal {
                children: l_children,
                ..
            } = &children[0]
            {
                assert_eq!(l_children.len(), 1);
                if let Node::Internal {
                    children: ll_children,
                    ..
                } = &l_children[0]
                {
                    assert_eq!(ll_children.len(), 2); // Contains L("05") and L("10")
                    assert_eq!(ll_children[0].key(), "05");
                    assert_eq!(ll_children[1].key(), "10");
                } else {
                    panic!("Expected internal node");
                }
            } else {
                panic!("Expected internal node");
            }

            // Inspect the right subtree
            if let Node::Internal {
                children: r_children,
                ..
            } = &children[1]
            {
                assert_eq!(r_children.len(), 2);
                let node1 = &r_children[0]; // I([L("15")])
                let node2 = &r_children[1]; // I([L("20"), L("30")])
                if let Node::Internal {
                    children: n1_children,
                    ..
                } = node1
                {
                    assert_eq!(n1_children.len(), 1);
                    assert_eq!(n1_children[0].key(), "15");
                } else {
                    panic!("Expected internal node");
                }
                if let Node::Internal {
                    children: n2_children,
                    ..
                } = node2
                {
                    assert_eq!(n2_children.len(), 2);
                    assert_eq!(n2_children[0].key(), "20");
                    assert_eq!(n2_children[1].key(), "30");
                } else {
                    panic!("Expected internal node");
                }
            } else {
                panic!("Expected internal node");
            }
        } else {
            panic!("Root should be internal");
        }
    }

    #[test]
    fn test_root_split() {
        let mut tree = MerkleSearchTree::new(2);
        tree.insert("10".to_string(), "v1".to_string());
        tree.insert("20".to_string(), "v2".to_string());
        // The root's children list is now [ L("10"), L("20"), L("30") ].

        tree.insert("30".to_string(), "v3".to_string()); // Triggers root split into two groups: [L("10")] and [L("20"), L("30")].

        let root_node = &tree.root;
        if let Node::Internal { children, .. } = root_node {
            assert_eq!(children.len(), 2);
            assert!(matches!(&children[0], Node::Internal { .. }));
            assert!(matches!(&children[1], Node::Internal { .. }));

            if let Node::Internal {
                children: left_children,
                ..
            } = &children[0]
            {
                assert_eq!(left_children.len(), 1);
                assert_eq!(left_children[0].key(), "10");
            } else {
                panic!("Child of root should be an internal node");
            }

            if let Node::Internal {
                children: right_children,
                ..
            } = &children[1]
            {
                assert_eq!(right_children.len(), 2);
                assert_eq!(right_children[0].key(), "20");
                assert_eq!(right_children[1].key(), "30");
            } else {
                panic!("Child of root should be an internal node");
            }
        } else {
            panic!("Root should be an internal node after splitting");
        }
    }

    #[test]
    fn test_hash_changes() {
        let mut tree = MerkleSearchTree::<String>::new(10);
        let initial_hash = tree.hash().clone();

        tree.insert("key1".to_string(), "value1".to_string());
        let hash_after_1 = tree.hash().clone();
        assert_ne!(initial_hash, hash_after_1);

        tree.insert("key2".to_string(), "value2".to_string());
        let hash_after_2 = tree.hash().clone();
        assert_ne!(hash_after_1, hash_after_2);
    }

    #[test]
    fn test_upsert_replaces_leaf() {
        let mut tree = MerkleSearchTree::<String>::new(10);
        tree.insert("key1".to_string(), "value1".to_string());

        // Check initial state
        let initial_hash = tree.hash().clone();
        if let Node::Internal { children, .. } = &tree.root {
            if let Node::Leaf { hash, .. } = &children[0] {
                let mut hasher = sha2::Sha256::new();
                hasher.update("value1".as_bytes());
                let expected_hash: [u8; 32] = hasher.finalize().into();
                assert_eq!(hash, &expected_hash);
            } else {
                panic!("Child should be a leaf");
            }
        } else {
            panic!("Root should be internal");
        }

        // Now, insert the same key with a new value
        tree.insert("key1".to_string(), "value1_modified".to_string());
        let updated_hash = tree.hash().clone();
        assert_ne!(initial_hash, updated_hash);

        // Check the updated state
        if let Node::Internal { children, .. } = &tree.root {
            // Should still only have one child
            assert_eq!(children.len(), 1);
            // Check that the hash of the leaf has changed
            if let Node::Leaf { hash, .. } = &children[0] {
                let mut hasher = sha2::Sha256::new();
                hasher.update("value1_modified".as_bytes());
                let expected_hash: [u8; 32] = hasher.finalize().into();
                assert_eq!(hash, &expected_hash);
            } else {
                panic!("Child should be a leaf");
            }
        } else {
            panic!("Root should be internal");
        }
    }
}
