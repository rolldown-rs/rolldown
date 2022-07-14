use ast::Id;
use ena::unify::{InPlaceUnificationTable, UnifyKey};
use hashbrown::HashMap;
use std::{fmt::Debug, hash::Hash, sync::Mutex};
use swc_atoms::JsWord;
use swc_common::{Mark, DUMMY_SP};
use swc_ecma_utils::quote_ident;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnaKey(u32);

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
pub struct UFriend<RealKey: Eq + Hash + Clone + Debug> {
    ena: Mutex<InPlaceUnificationTable<EnaKey>>,
    ena_key_to_real_key: HashMap<EnaKey, RealKey>,
    real_key_to_ena_key: HashMap<RealKey, EnaKey>,
    // stored: Vec<Key>,
}

impl<Key: Eq + Hash + Clone + Debug> UFriend<Key> {
    pub fn new() -> Self {
        Self {
            ena: Default::default(),
            ena_key_to_real_key: Default::default(),
            real_key_to_ena_key: Default::default(),
            // stored: Default::default(),
        }
    }

    pub fn add_key(&mut self, real_key: Key) {
        self.real_key_to_ena_key
            .entry(real_key)
            .or_insert_with_key(|key| {
                let ena_key = self.ena.get_mut().unwrap().new_key(());
                self.ena_key_to_real_key.insert(ena_key, key.clone());
                ena_key
            });
    }

    pub fn union(&self, key1: &Key, key2: &Key) {
        let ena_key1 = self.real_key_to_ena_key.get(key1).unwrap_or_else(|| {
            panic!("Key {:?} not found for pair ({:?}, {:?})", key1, key1, key2)
        });
        let ena_key2 = self.real_key_to_ena_key.get(key2).unwrap_or_else(|| {
            panic!("Key {:?} not found for pair ({:?}, {:?})", key2, key1, key2)
        });
        self.ena.lock().unwrap().union(*ena_key1, *ena_key2);
    }

    pub fn unioned(&self, key1: &Key, key2: &Key) -> bool {
        let ena_key1 = self.real_key_to_ena_key.get(key1).unwrap();
        let ena_key2 = self.real_key_to_ena_key.get(key2).unwrap();
        self.ena.lock().unwrap().unioned(*ena_key1, *ena_key2)
    }

    pub fn find_root(&self, key: &Key) -> Option<&Key> {
        let ena_key = self.real_key_to_ena_key.get(key)?;
        let ena_root = self.ena.lock().unwrap().find(*ena_key);
        self.ena_key_to_real_key.get(&ena_root)
    }
}

impl UFriend<Id> {
    pub fn new_id(&mut self, name: JsWord) -> Id {
        let id = quote_ident!(DUMMY_SP.apply_mark(Mark::new()), name).to_id();
        self.add_key(id.clone());
        id
    }
}
