use crate::RawEntryId;

pub(crate) struct Components {
    parents: Vec<RawEntryId>,
}

impl Components {
    pub(crate) fn new(ids: impl Iterator<Item = RawEntryId>) -> Self {
        Self {
            parents: ids.collect(),
        }
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "Callers initialize components from all source-order entry IDs and query IDs from that same snapshot. Union preserves those in-bounds, acyclic parent indices."
    )]
    pub(crate) fn root(&mut self, mut id: RawEntryId) -> RawEntryId {
        while self.parents[id.index()] != id {
            let parent = self.parents[id.index()];
            self.parents[id.index()] = self.parents[parent.index()];
            id = self.parents[id.index()];
        }
        id
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "Both roots belong to this component vector. Linking the larger source occurrence to the smaller preserves valid, acyclic parent indices."
    )]
    pub(crate) fn join(&mut self, left: RawEntryId, right: RawEntryId) {
        let left = self.root(left);
        let right = self.root(right);
        let (first, second) = if left.index() < right.index() {
            (left, right)
        } else {
            (right, left)
        };
        self.parents[second.index()] = first;
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "The loop traverses the vector's exact bounds. Every stored parent is an entry ID from this snapshot, and root preserves that invariant."
    )]
    pub(crate) fn freeze(mut self) -> Vec<RawEntryId> {
        for index in 0..self.parents.len() {
            self.parents[index] = self.root(self.parents[index]);
        }
        self.parents
    }
}
