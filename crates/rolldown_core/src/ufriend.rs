use ena::unify::{InPlaceUnificationTable, UnifyKey};
use std::{collections::HashMap, hash::Hash, fmt::Debug};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EnaKey(u32);

impl UnifyKey for EnaKey {
    type Value = ();

    fn index(&self) -> u32 {
        self.0
    }

    fn from_index(u: u32) -> Self {
        EnaKey(u)
    }

    fn tag() -> &'static str {
        "EnaKey"
    }
}

#[derive(Debug)]
pub struct UFriend<Key: Eq + Hash + Clone + Debug> {
    ena: InPlaceUnificationTable<EnaKey>,
    ena_key_to_index: HashMap<EnaKey, Key>,
    index_to_ena_key_map: HashMap<Key, EnaKey>,
    // stored: Vec<Key>,
}

impl<Key: Eq + Hash + Clone + Debug> UFriend<Key> {
    pub fn new() -> Self {
        Self {
            ena: Default::default(),
            ena_key_to_index: Default::default(),
            index_to_ena_key_map: Default::default(),
            // stored: Default::default(),
        }
    }

    pub fn add_key(&mut self, key: Key) {
      if !self.index_to_ena_key_map.contains_key(&key) {
        // self.stored.push(key);
        // let index = self.stored.len() - 1;
        let ena_key = self.ena.new_key(());
        // self.ena_key_to_key_map.insert(ena_key, key);
        self.index_to_ena_key_map.insert(key.clone(), ena_key);
        self.ena_key_to_index.insert(ena_key, key);
      }
    }

    pub fn union(&mut self, key1: &Key, key2: &Key) {
        let ena_key1 = self.index_to_ena_key_map.get(key1).unwrap();
        let ena_key2 = self.index_to_ena_key_map.get(key2).unwrap();
        self.ena.union(*ena_key1, *ena_key2);
    }

    pub fn unioned(&mut self, key1: &Key, key2: &Key) -> bool {
        let ena_key1 = self.index_to_ena_key_map.get(key1).unwrap();
        let ena_key2 = self.index_to_ena_key_map.get(key2).unwrap();
        self.ena.unioned(*ena_key1, *ena_key2)
    }

    pub fn asset_find_root(&mut self, key: &Key) -> &Key {
        let ena_key = self.index_to_ena_key_map.get(key).unwrap_or_else(|| panic!("key: {:?}", key));
        let ena_root = self.ena.find(*ena_key);
        &self.ena_key_to_index[&ena_root]
    }

    pub fn find_root(&mut self, key: &Key) -> Option<&Key> {
        let ena_key = self.index_to_ena_key_map.get(key)?;
        let ena_root = self.ena.find(*ena_key);
        self.ena_key_to_index.get(&ena_root)
    }
}
