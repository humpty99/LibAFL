//! In-memory corpus, keeps all test cases in memory at all times

use core::cell::RefCell;

use serde::{Deserialize, Serialize};

use crate::{
    corpus::{Corpus, CorpusId, Testcase},
    inputs::{Input, UsesInput},
    Error,
};

#[cfg(not(feature = "corpus_btreemap"))]
pub struct TestcaseStorageItem<I> where
    I: Input, {
    pub testcase: RefCell<Testcase<I>>,
    pub prev: Option<CorpusId>,
    pub next: Option<CorpusId>
}

#[cfg(not(feature = "corpus_btreemap"))]
/// The map type in which testcases are stored (enable the feature 'corpus_btreemap' to use a `BTreeMap` instead of `HashMap`)
pub type TestcaseStorageMap<I> = hashbrown::HashMap<CorpusId, TestcaseStorageItem<I>>;

#[cfg(feature = "corpus_btreemap")]
/// The map type in which testcases are stored (disable the feature 'corpus_btreemap' to use a `HashMap` instead of `BTreeMap`)
pub type TestcaseStorageMap<I> =
    alloc::collections::btree_map::BTreeMap<CorpusId, RefCell<Testcase<I>>>;

/// Storage map for the testcases (used in `Corpus` implementations) with an incremental index
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
#[serde(bound = "I: serde::de::DeserializeOwned")]
pub struct TestcaseStorage<I>
where
    I: Input,
{
    /// The map in which testcases are stored
    pub map: TestcaseStorageMap<I>,
    /// The progressive idx
    progressive_idx: usize,
    /// First inserted idx
    #[cfg(not(feature = "corpus_btreemap"))]
    first_idx: Option<CorpusId>,
    /// Last inserted idx
    #[cfg(not(feature = "corpus_btreemap"))]
    last_idx: Option<CorpusId>,
}

impl<I> UsesInput for TestcaseStorage<I>
where
    I: Input,
{
    type Input = I;
}

impl<I> TestcaseStorage<I>
where
    I: Input,
{
    /// Insert a testcase assigning a `CorpusId` to it
    #[cfg(not(feature = "corpus_btreemap"))]
    pub fn insert(&mut self, testcase: RefCell<Testcase<I>>) -> CorpusId {
        let idx = CorpusId::from(self.progressive_idx);
        self.progressive_idx += 1;
        let prev = if let Some(last_idx) = self.last_idx {
            self.map.get_mut(&last_idx).unwrap().next = Some(idx);
            Some(last_idx)
        } else {
            None
        };
        if self.first_idx.is_none() {
            self.first_idx = Some(idx);
        }
        self.last_idx = Some(idx);
        self.map.insert(idx, TestcaseStorageItem { testcase, prev, next: None });
        idx
    }

    /// Insert a testcase assigning a `CorpusId` to it
    #[cfg(feature = "corpus_btreemap")]
    pub fn insert(&mut self, testcase: RefCell<Testcase<I>>) -> CorpusId {
        let idx = CorpusId::from(self.progressive_idx);
        self.progressive_idx += 1;
        self.map.insert(idx, testcase);
        idx
    }
    
    #[cfg(not(feature = "corpus_btreemap"))]
    pub fn remove(&self, idx: CorpusId) -> Option<&RefCell<Testcase<I>>> {
        if let Some(item) = self.map.remove(&idx) {
            if let Some(prev) = item.prev {
                self.map.get(&prev).unwrap().next = item.next;
            } else {
                // first elem
                self.first_idx = item.next;
            }
            if let Some(next) = item.next {
                self.map.get(&next).unwrap().prev = item.prev;
            } else {
                // last elem
                self.last_idx = item.prev;
            }
            Some(item)
        } else {
            None
        }
    }

    #[cfg(feature = "corpus_btreemap")]
    pub fn remove(&self, idx: CorpusId) -> Option<&RefCell<Testcase<I>>> {
        self.map.remove(&idx)
    }

    #[cfg(not(feature = "corpus_btreemap"))]
    pub fn get(&self, idx: CorpusId) -> Option<&RefCell<Testcase<I>>> {
        self.map.get(&idx)
    }

    #[cfg(feature = "corpus_btreemap")]
    pub fn get(&self, idx: CorpusId) -> Option<&RefCell<Testcase<I>>> {
        self.map.get(&idx).map(|x| x.testcase)
    }

    #[cfg(not(feature = "corpus_btreemap"))]
    fn next(&self, idx: CorpusId) -> Option<CorpusId> {
        if let Some(item) = self.map.get(idx) {
            item.next
        } else {
            None
        }
    }

    #[cfg(feature = "corpus_btreemap")]
    fn next(&self, idx: CorpusId) -> Option<CorpusId> {
        let mut range = self.map.range(core::ops::Bound::Included(idx), core::ops::Bound::Unbounded);
        if let Some((this_id, _)) = range.next() {
            if idx != this_id {
                return None;
            }
        }
        if let Some((next_id, _)) = range.next() {
            Some(next_id)
        } else {
            None
        }
    }

    #[cfg(not(feature = "corpus_btreemap"))]
    fn prev(&self, idx: CorpusId) -> Option<CorpusId> {
        if let Some(item) = self.map.get(idx) {
            item.prev
        } else {
            None
        }
    }

    #[cfg(feature = "corpus_btreemap")]
    fn prev(&self, idx: CorpusId) -> Option<CorpusId> {
        let mut range = self.map.range(core::ops::Bound::Unbounded, core::ops::Bound::Included(idx));
        if let Some((this_id, _)) = range.next_back() {
            if idx != this_id {
                return None;
            }
        }
        if let Some((prev_id, _)) = range.next_back() {
            Some(prev_id)
        } else {
            None
        }
    }

    #[cfg(not(feature = "corpus_btreemap"))]
    fn first(&self) -> Option<CorpusId> {
        self.first_idx
    }

    #[cfg(feature = "corpus_btreemap")]
    fn first(&self) -> Option<CorpusId> {
        self.map.iter().next()
    }

    #[cfg(not(feature = "corpus_btreemap"))]
    fn last(&self) -> Option<CorpusId> {
        self.last_idx
    }

    #[cfg(feature = "corpus_btreemap")]
    fn last(&self) -> Option<CorpusId> {
        self.map.iter().next_back()
    }

    /// Create new
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

/// A corpus handling all in memory.
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
#[serde(bound = "I: serde::de::DeserializeOwned")]
pub struct InMemoryCorpus<I>
where
    I: Input,
{
    entries: TestcaseStorage<I>,
    current: Option<CorpusId>,
}

impl<I> UsesInput for InMemoryCorpus<I>
where
    I: Input,
{
    type Input = I;
}

impl<I> Corpus for InMemoryCorpus<I>
where
    I: Input,
{
    /// Returns the number of elements
    #[inline]
    fn count(&self) -> usize {
        self.entries.map.len()
    }

    /// Add an entry to the corpus and return its index
    #[inline]
    fn add(&mut self, testcase: Testcase<I>) -> Result<usize, Error> {
        Ok(self.entries.insert(RefCell::new(testcase)))
    }

    /// Replaces the testcase at the given idx
    #[inline]
    fn replace(&mut self, idx: CorpusId, testcase: Testcase<I>) -> Result<Testcase<I>, Error> {
        if let Some(entry) = self.entries.map.get_mut(&idx) {
            Ok(entry.replace(testcase))
        } else {
            Err(Error::key_not_found(format!("Index {idx} not found")))
        }
    }

    /// Removes an entry from the corpus, returning it if it was present.
    #[inline]
    fn remove(&mut self, idx: CorpusId) -> Result<Option<Testcase<I>>, Error> {
        Ok(self.entries.map.remove(&idx).map(|x| x.take()))
    }

    /// Get by id
    #[inline]
    fn get(&self, idx: CorpusId) -> Result<&RefCell<Testcase<I>>, Error> {
        self.entries
            .map
            .get(&idx)
            .ok_or_else(|| Error::key_not_found(format!("Index {idx} not found")))
    }

    /// Current testcase scheduled
    #[inline]
    fn current(&self) -> &Option<CorpusId> {
        &self.current
    }

    /// Current testcase scheduled (mutable)
    #[inline]
    fn current_mut(&mut self) -> &mut Option<CorpusId> {
        &mut self.current
    }
    
    #[inline]
    fn next(&self, idx: CorpusId) -> Option<CorpusId> {
        self.storage.next(idx)
    }

    #[inline]
    fn prev(&self, idx: CorpusId) -> Option<CorpusId> {
        self.storage.prev(idx)
    }

    #[inline]
    fn first(&self) -> Option<CorpusId> {
        self.storage.first()
    }

    #[inline]
    fn last(&self) -> Option<CorpusId> {
        self.storage.last()
    }
}

impl<I> InMemoryCorpus<I>
where
    I: Input,
{
    /// Creates a new [`InMemoryCorpus`], keeping all [`Testcase`]`s` in memory.
    /// This is the simplest and fastest option, however test progress will be lost on exit or on OOM.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: TestcaseStorage::new(),
            current: None,
        }
    }
}

/// `InMemoryCorpus` Python bindings
#[cfg(feature = "python")]
pub mod pybind {
    use pyo3::prelude::*;
    use serde::{Deserialize, Serialize};

    use crate::{
        corpus::{pybind::PythonCorpus, InMemoryCorpus},
        inputs::BytesInput,
    };

    #[pyclass(unsendable, name = "InMemoryCorpus")]
    #[allow(clippy::unsafe_derive_deserialize)]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    /// Python class for InMemoryCorpus
    pub struct PythonInMemoryCorpus {
        /// Rust wrapped InMemoryCorpus object
        pub inner: InMemoryCorpus<BytesInput>,
    }

    #[pymethods]
    impl PythonInMemoryCorpus {
        #[new]
        fn new() -> Self {
            Self {
                inner: InMemoryCorpus::new(),
            }
        }

        fn as_corpus(slf: Py<Self>) -> PythonCorpus {
            PythonCorpus::new_in_memory(slf)
        }
    }
    /// Register the classes to the python module
    pub fn register(_py: Python, m: &PyModule) -> PyResult<()> {
        m.add_class::<PythonInMemoryCorpus>()?;
        Ok(())
    }
}
