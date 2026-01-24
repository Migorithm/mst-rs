use sha2::Digest;
use std::{
    cmp::Ordering,
    ops::{Deref, DerefMut},
};

// The public interface to the tree
pub struct MerkleSearchTree<K> {
    root: Node<K>,
    max_children: usize,
}

// The internal and leaf nodes of the tree

enum Node<K> {
    Internal {
        hash: NodeHash,
        children: Vec<Node<K>>,
        max_key: K,
    },
    Leaf {
        key: K,
        hash: NodeHash,
    },
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct NodeHash([u8; 32]);
impl From<[u8; 32]> for NodeHash {
    fn from(value: [u8; 32]) -> Self {
        NodeHash(value)
    }
}
impl Deref for NodeHash {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NodeHash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K: Default> Default for Node<K> {
    fn default() -> Self {
        Node::Internal {
            hash: NodeHash([0; 32]),
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
        let hashed: [u8; 32] = hasher.finalize().into();
        let hash = hashed.into();

        let leaf = Node::Leaf { key, hash };

        if let Some(new_sibling) = self.root.insert(leaf, self.max_children) {
            // The root split, so we need to create a new root.
            let old_root = std::mem::take(&mut self.root);

            let mut new_root = Node::Internal {
                hash,
                children: vec![old_root, new_sibling],
                max_key: K::default(), // Will be set by recalculate
            };
            new_root.recalculate();
            self.root = new_root;
        }
    }

    pub fn hash(&self) -> &NodeHash {
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

    fn hash(&self) -> &NodeHash {
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
            *hash = Default::default();
            if let Some(last_child) = children.last() {
                *max_key = last_child.key().clone();
                for child in children {
                    xor_assign(hash, child.hash());
                }
            }
        }
    }

    // Inserts a new node into the subtree.
    // Returns a new sibling if the current node splits.
    fn insert(&mut self, new_node: Node<K>, max_children: usize) -> Option<Node<K>> {
        // This method is only callable on Node::Internal

        let Node::Internal {
            hash: self_hash,
            children,
            max_key,
        } = self
        else {
            panic!("Cannot insert into a leaf node.")
        };

        // Decide whether to descend further or insert at this level.
        // descend if our children are also Internal nodes.
        // insert here if our children are Leaves (or if we have no children yet).
        let are_children_leaves = children.is_empty() || !children[0].is_internal();

        if are_children_leaves {
            //  Base Case: children are leaves. Handle insert/upsert.
            match children.binary_search(&new_node) {
                Ok(index) => {
                    xor_assign(self_hash, children[index].hash());
                    children[index] = new_node;
                    xor_assign(self_hash, children[index].hash());
                }
                Err(index) => {
                    // Key not found. Insert the new leaf.
                    children.insert(index, new_node);
                    xor_assign(self_hash, children[index].hash());
                }
            }
        } else {
            // ! Recursive case - children are Internal nodes
            // Find which child to descend into.
            let mut child_index = children.partition_point(|child| child.key() < new_node.key());

            // If the new key is larger than all existing children, partition_point
            // returns children.len(). In this case, we route it to the last child.
            if child_index == children.len() {
                child_index = children.len() - 1;
            }

            let old_child_hash = *children[child_index].hash();

            // Descend and get a potential new sibling from the child if it splits.
            let new_sibling_from_child = children[child_index].insert(new_node, max_children);

            let new_child_hash = *children[child_index].hash();
            xor_assign(self_hash, &old_child_hash);
            xor_assign(self_hash, &new_child_hash);

            // If the child split, add its new sibling to our children list.
            if let Some(new_sibling) = new_sibling_from_child {
                let insert_at = child_index + 1;
                children.insert(insert_at, new_sibling);
                xor_assign(self_hash, children[insert_at].hash());
            }
        }

        // After insertion, check if it needs to split itself.
        if children.len() > max_children {
            let mid = children.len() / 2;
            let sibling_children = children.split_off(mid);
            let mut new_sibling = Node::Internal {
                hash: Default::default(),
                children: sibling_children,
                max_key: K::default(), // will be recalculated
            };
            new_sibling.recalculate();
            xor_assign(self_hash, new_sibling.hash());
            if let Some(last) = children.last() {
                *max_key = last.key().clone();
            }

            Some(new_sibling)
        } else {
            if let Some(last) = children.last() {
                *max_key = last.key().clone();
            }
            None
        }
    }
}

#[inline]
fn xor_assign(target: &mut NodeHash, source: &NodeHash) {
    for (t, s) in target.iter_mut().zip(source.iter()) {
        *t ^= s;
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
    fn test_update_existing_key() {
        let mut tree = MerkleSearchTree::new(4);

        tree.insert(1, "version_1".to_string());
        let hash_v1 = *tree.hash();

        tree.insert(1, "version_2".to_string()); // Update the value for key 1
        let hash_v2 = *tree.hash();

        assert_ne!(hash_v1, hash_v2, "Updating a value should change the hash");
    }

    #[test]
    fn test_merkle_property() {
        let mut tree1 = MerkleSearchTree::new(4);
        tree1.insert(1, "apple".to_string());
        tree1.insert(2, "banana".to_string());

        let mut tree2 = MerkleSearchTree::new(4);
        tree2.insert(1, "apple".to_string());
        tree2.insert(2, "banana".to_string());

        assert_eq!(
            tree1.hash(),
            tree2.hash(),
            "Identical content should yield identical hashes"
        );

        // Modify tree2
        tree2.insert(3, "cherry".to_string());
        assert_ne!(
            tree1.hash(),
            tree2.hash(),
            "Different content must yield different hashes"
        );

        // Add same content to tree1
        tree1.insert(3, "cherry".to_string());
        assert_eq!(tree1.hash(), tree2.hash(), "Trees should match again");
    }

    #[test]
    fn test_insert_largest_key_fix() {
        // This test specifically targets the panic we fixed:
        // "index out of bounds" when inserting a key larger than all current children.
        let mut tree = MerkleSearchTree::new(2);

        // 1. Insert base keys
        tree.insert(10, "v10".to_string());
        tree.insert(20, "v20".to_string());

        // 2. Force a split (max_children = 2), creating a deeper tree
        // The tree should now have Internal nodes.
        tree.insert(30, "v30".to_string());

        // 3. Insert a key strictly larger than the current max_key (30)
        // If the `partition_point` fix is missing, this lines panics.
        tree.insert(40, "v40".to_string());

        // Verify no panic and structure is sound
        assert_ne!(tree.hash(), &Default::default());
    }
}
