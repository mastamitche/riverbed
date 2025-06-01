use std::{collections::HashMap, hash::Hash, ops::Index, slice::Iter};

#[derive(Debug)]
pub struct Palette<E: Hash + Eq + PartialEq + Clone> {
    leftmap: HashMap<E, usize>,
    rightmap: Vec<E>,
}

impl<E: Hash + Eq + PartialEq + Clone> Palette<E> {
    pub fn new() -> Self {
        Self {
            leftmap: HashMap::new(),
            rightmap: Vec::new(),
        }
    }

    pub fn iter(&self) -> Iter<'_, E> {
        self.rightmap.iter()
    }

    // Get a reference to the rightmap
    pub fn elements(&self) -> &Vec<E> {
        &self.rightmap
    }

    pub fn index(&mut self, elem: E) -> usize {
        *self.leftmap.entry(elem.clone()).or_insert_with(|| {
            self.rightmap.push(elem);
            self.rightmap.len() - 1
        })
    }
    pub fn len(&self) -> usize {
        self.leftmap.len()
    }

    pub fn get_all_elements(&self) -> Vec<E> {
        self.rightmap.clone()
    }

    // Create a palette from an existing collection of elements
    pub fn from_elements(elements: Vec<E>) -> Self {
        let mut palette = Self {
            leftmap: HashMap::with_capacity(elements.len()),
            rightmap: elements,
        };

        // Build the leftmap
        for (idx, elem) in palette.rightmap.iter().enumerate() {
            palette.leftmap.insert(elem.clone(), idx);
        }

        palette
    }
}

impl<E: Hash + Eq + PartialEq + Clone> Index<usize> for Palette<E> {
    type Output = E;

    fn index(&self, index: usize) -> &Self::Output {
        &self.rightmap[index]
    }
}
