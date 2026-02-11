// Disable indexing lints for now
#![allow(clippy::indexing_slicing)]

// TODO: replace with `use creusot_contracts::prelude::Default;` (or uncomment `/* Default, */` below)
// after fixing Creusot's Default derive macro to support private fields,
// or just replace with handwritten impl Default with some specs.
use core::mem;
use creusot_std::{
    logic::ops::NthBitLogic,
    model,
    prelude::{Clone, PartialEq, /* Default, */ Int, *},
    std::iter::IteratorSpec,
};
use std::cmp::Ordering;
use std::default::Default;

use self::clist::CList;

pub const USIZE_BITS: usize = mem::size_of::<usize>() * 8;

/// Runqueue number.
#[derive(Copy, Debug, PartialEq, Eq /*PartialOrd, Ord*/)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RunqueueId(u8);

#[allow(clippy::non_canonical_clone_impl)]
impl Clone for RunqueueId {
    #[ensures(*self == result)]
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for RunqueueId {
    #[ensures(result == Some((*self).deep_model().cmp_log((*other).deep_model())))]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for RunqueueId {
    #[ensures(result == (*self).deep_model().cmp_log((*other).deep_model()))]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl RunqueueId {
    /// Wraps the given ID as a [`RunqueueId`].
    #[must_use]
    #[ensures(result@ == value@)]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }
}

impl From<RunqueueId> for usize {
    fn from(value: RunqueueId) -> Self {
        usize::from(value.0)
    }
}

impl DeepModel for RunqueueId {
    type DeepModelTy = Int;
    #[logic]
    #[trusted]
    fn deep_model(self) -> Self::DeepModelTy {
        self.0.deep_model()
    }
}

impl model::View for RunqueueId {
    type ViewTy = Int;

    #[logic]
    fn view(self) -> Self::ViewTy {
        pearlite! { self.0@ }
    }
}

impl OrdLogic for RunqueueId {
    #[logic]
    #[ensures(result == Int::cmp_log(self@, other@))]
    fn cmp_log(self, other: Self) -> core::cmp::Ordering {
        pearlite! { Int::cmp_log(self@, other@) }
    }

    #[logic(law)]
    #[ensures(x.lt_log(y) == (x.cmp_log(y) == Ordering::Less))]
    fn cmp_lt_log(x: Self, y: Self) {}

    #[logic(law)]
    #[ensures(x.le_log(y) == (x.cmp_log(y) != Ordering::Greater))]
    fn cmp_le_log(x: Self, y: Self) {}

    #[logic(law)]
    #[ensures(x.ge_log(y) == (x.cmp_log(y) != Ordering::Less))]
    fn cmp_ge_log(x: Self, y: Self) {}

    #[logic(law)]
    #[ensures(x.gt_log(y) == (x.cmp_log(y) == Ordering::Greater))]
    fn cmp_gt_log(x: Self, y: Self) {}

    #[logic(law)]
    #[ensures(x.cmp_log(x) == Ordering::Equal)]
    fn refl(x: Self) {}

    #[logic(law)]
    #[requires(x.cmp_log(y) == o)]
    #[requires(y.cmp_log(z) == o)]
    #[ensures(x.cmp_log(z) == o)]
    fn trans(x: Self, y: Self, z: Self, o: core::cmp::Ordering) {}

    #[logic(law)]
    #[requires(x.cmp_log(y) == Ordering::Less)]
    #[ensures(y.cmp_log(x) == Ordering::Greater)]
    fn antisym1(x: Self, y: Self) {}

    #[logic(law)]
    #[requires(x.cmp_log(y) == Ordering::Greater)]
    #[ensures(y.cmp_log(x) == Ordering::Less)]
    fn antisym2(x: Self, y: Self) {}

    #[logic(law)]
    #[ensures((x == y) == (x.cmp_log(y) == Ordering::Equal))]
    fn eq_cmp(x: Self, y: Self) {}
}

/// Identifier of a thread.
#[derive(Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ThreadId(u8);

impl Clone for ThreadId {
    #[check(ghost)]
    #[ensures(*self == result)]
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl ThreadId {
    /// Wraps the given ID as a [`ThreadId`].
    #[must_use]
    #[ensures(result@ == value@)]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }
}

impl From<ThreadId> for usize {
    fn from(value: ThreadId) -> Self {
        usize::from(value.0)
    }
}

impl DeepModel for ThreadId {
    type DeepModelTy = Int;
    #[logic]
    #[trusted]
    fn deep_model(self) -> Self::DeepModelTy {
        pearlite! { self.0@ }
    }
}

impl model::View for ThreadId {
    type ViewTy = Int;

    #[logic]
    fn view(self) -> Self::ViewTy {
        pearlite! {
            self.0@
        }
    }
}

/// Runqueue for `N_QUEUES`, supporting `N_THREADS` total.
///
/// Assumptions:
/// - runqueue numbers (corresponding priorities) are `0..N_QUEUES` (exclusive)
/// - higher runqueue number ([`RunqueueId`]) means higher priority
/// - runqueue numbers fit in usize bits (supporting max 32 priority levels)
/// - [`ThreadId`]s range from `0..N_THREADS`
/// - `N_THREADS` is <255 (as u8 is used to store them, but 0xFF is used as
///   special value)
///
/// The current implementation needs an usize for the bit cache,
/// an `[u8; N_QUEUES]` array for the list tail indexes
/// and an `[u8; N_THREADS]` for the list next indexes.
pub struct RunQueue<const N_QUEUES: usize, const N_THREADS: usize> {
    /// Bitcache that represents the currently used queues
    /// in `0..N_QUEUES`.
    bitcache: usize,
    queues: clist::CList<N_QUEUES, N_THREADS>,
}

#[cfg(not(creusot))]
impl<const N_QUEUES: usize, const N_THREADS: usize> Default for RunQueue<N_QUEUES, N_THREADS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N_QUEUES: usize, const N_THREADS: usize> Invariant for RunQueue<N_QUEUES, N_THREADS> {
    #[logic]
    fn invariant(self) -> bool {
        pearlite! {
            self.bitcache@ < usize::MAX@
                && self.queues.invariant()
                && N_QUEUES@ < USIZE_BITS@
                && N_QUEUES@ < 64 && N_THREADS@ < 64
                && valid_cache(self.bitcache, N_QUEUES)
        }
    }
}

impl<const N_QUEUES: usize, const N_THREADS: usize> RunQueue<{ N_QUEUES }, { N_THREADS }> {
    #[logic]
    pub fn valid_rq_id(id: Int) -> bool {
        CList::<N_QUEUES, N_THREADS>::valid_rq_id(id)
    }

    #[logic]
    pub fn valid_th_id(id: Int) -> bool {
        CList::<N_QUEUES, N_THREADS>::valid_th_id(id)
    }

    #[logic]
    pub fn valid_rq(&self, id: Int) -> bool {
        self.queues.valid_rq(id)
    }

    #[inline(always)]
    #[trusted]
    #[bitwise_proof]
    #[requires(Self::valid_rq_id(id@))]
    #[ensures(result.nth_bit(USIZE_BITS@ - id@ - 1))]
    #[ensures(forall<i: Int> 0 <= i && i < USIZE_BITS@ && i != USIZE_BITS@ - id@ - 1 ==> !result.nth_bit(i))]
    #[ensures(valid_cache(result, N_QUEUES))]
    fn mask_of_id(id: u8) -> usize {
        1 << id
    }

    #[inline(always)]
    #[trusted]
    #[bitwise_proof]
    #[requires(Self::valid_rq_id(rq@))]
    fn unset_bit_rq(&mut self, rq: u8) {
        self.bitcache &= !(Self::mask_of_id(rq))
    }

    #[inline(always)]
    #[trusted]
    #[bitwise_proof]
    #[requires(Self::valid_rq_id(rq@))]
    #[ensures(self.valid_rq(rq@) ==> (^self).valid_rq(rq@))]
    fn set_bit_rq(&mut self, rq: u8) {
        self.bitcache |= Self::mask_of_id(rq)
    }

    /// Returns a new [`RunQueue`].
    #[must_use]
    #[requires(N_QUEUES@ < 64)]
    #[requires(N_THREADS@ < 64)]
    pub const fn new() -> RunQueue<{ N_QUEUES }, { N_THREADS }> {
        // unfortunately we cannot assert!() on N_QUEUES and N_THREADS,
        // as panics in const fn's are not (yet) implemented.
        RunQueue {
            bitcache: 0,
            queues: CList::new(),
        }
    }

    /// Adds thread with tid `n` to runqueue number `rq`.
    #[requires(self.valid_rq(rq@))]
    #[requires(Self::valid_th_id(n@))]
    pub fn add(&mut self, n: ThreadId, rq: RunqueueId) {
        #[cfg(not(creusot))]
        {
            debug_assert!(usize::from(n) < N_THREADS);
            debug_assert!(usize::from(rq) < N_QUEUES);
        }
        //self.bitcache |= 1 << rq.0;
        self.set_bit_rq(rq.0);
        self.queues.push(n.0, rq.0);
    }

    /// Returns the head of the runqueue without removing it.
    #[requires(Self::valid_rq_id(rq@))]
    pub fn peek_head(&self, rq: RunqueueId) -> Option<ThreadId> {
        self.queues.peek_head(rq.0).map(ThreadId::new)
    }

    /// Removes thread with tid `n` from runqueue number `rq`.
    ///
    /// # Panics
    ///
    /// Panics if `n` is not the queue's head.
    /// This is fine, Ariel OS only ever calls `pop_head()` for the current thread.
    #[requires(Self::valid_rq_id(rq@))]
    #[requires(Self::valid_th_id(n@))]
    #[requires(match self.head(rq) {
            Some(i) => i@ == rq@,
            None    => false
        }
    )]
    pub fn pop_head(&mut self, n: ThreadId, rq: RunqueueId) {
        #[cfg(not(creusot))]
        {
            debug_assert!(usize::from(n) < N_THREADS);
            debug_assert!(usize::from(rq) < N_QUEUES)
        };
        proof_assert!(n@ < N_THREADS@);
        proof_assert!(rq@ < N_QUEUES@);
        let popped = self.queues.pop_head(rq.0);
        //
        proof_assert!({
            match popped {
                Some(i) => i@ == rq@,
                None    => false
            }
        });
        #[cfg(not(creusot))]
        assert_eq!(popped, Some(n.0));
        if self.queues.is_empty(rq.0) {
            //self.bitcache &= !(1 << rq.0);
            self.unset_bit_rq(rq.0);
        }
    }

    #[logic]
    pub fn head(self, rq: RunqueueId) -> Option<u8> {
        self.queues.head(rq.0)
    }

    /// Removes thread with tid `n`.
    #[requires(Self::valid_th_id(n@))]
    pub fn del(&mut self, n: ThreadId) {
        if let Some(empty_runqueue) = self.queues.del(n.0) {
            //self.bitcache &= !(1 << empty_runqueue);
            self.unset_bit_rq(empty_runqueue);
        }
    }

    /// Returns the tid that should run next.
    ///
    /// Returns the next runnable thread of
    /// the runqueue with the highest index.
    #[must_use]
    pub fn get_next(&self) -> Option<ThreadId> {
        self.get_next_with_rq().map(|(tid, _)| tid)
    }

    /// Returns the tid that should run next and the runqueue it is in.
    #[must_use]
    #[ensures(
        match result {
            Some((tid, qid)) =>
                Self::valid_th_id(tid@) && Self::valid_rq_id(qid@),
            None => true
        }
    )]
    pub fn get_next_with_rq(&self) -> Option<(ThreadId, RunqueueId)> {
        let rq_ffs = ffs(self.bitcache);
        if rq_ffs == 0 {
            return None;
        }
        let rq = rq_ffs as u8 - 1;
        self.queues
            .peek_head(rq)
            .map(|id| (ThreadId::new(id), RunqueueId::new(rq)))
    }

    /// Returns the next thread from the runqueue that fulfills the predicate.
    #[trusted]
    #[requires(forall<x: &ThreadId> predicate.precondition((x,), ))]
    pub fn get_next_filter<F: FnMut(&ThreadId) -> bool>(
        &self,
        mut predicate: F,
    ) -> Option<ThreadId> {
        //let (next, prio) = self.get_next_with_rq()?;   // broken syntax
        if let Some((next, prio)) = self.get_next_with_rq() {
            if predicate(&next) {
                return Some(next);
            }
            // impossible precondition (no spec for find ?)
            self.iter_from(next, prio).find(predicate)
        } else {
            None
        }
    }

    /// Pop the thread that should run next.
    ///
    /// Pops the next runnable thread of
    /// the runqueue with the highest index.
    pub fn pop_next(&mut self) -> Option<ThreadId> {
        let rq_ffs = ffs(self.bitcache);
        if rq_ffs == 0 {
            return None;
        }
        proof_assert!(Self::valid_rq_id(rq_ffs@));
        let rq = (rq_ffs - 1) as u8;

        let head = self.queues.pop_head(rq).map(ThreadId::new);
        if self.queues.is_empty(rq) {
            //self.bitcache &= !(1 << rq);
            self.unset_bit_rq(rq);
        }
        head
    }

    /// Advances runqueue number `rq`.
    ///
    /// This is used to "yield" to another thread of *the same* priority.
    ///
    /// Returns `false` if the operation had no effect, i.e. when the runqueue
    /// is empty or only contains a single thread.
    #[requires(self.valid_rq(rq@))]
    pub fn advance(&mut self, rq: RunqueueId) -> bool {
        #[cfg(not(creusot))]
        debug_assert!((usize::from(rq)) < N_QUEUES);
        proof_assert!(rq@ < N_QUEUES@);
        self.queues.advance(rq.0)
    }

    /// Checks if a runqueue is empty.
    #[requires(Self::valid_rq_id(rq@))]
    pub fn is_empty(&self, rq: RunqueueId) -> bool {
        #[cfg(not(creusot))]
        debug_assert!((rq.0 as usize) < N_QUEUES);
        proof_assert!(rq.0@ < N_QUEUES@);
        self.queues.is_empty(rq.0)
    }

    /// Returns an iterator over the [`RunQueue`], starting after thread `start` in runqueue `rq`.
    ///
    /// The `start` is not included in the iterator.
    #[must_use]
    #[requires(Self::valid_th_id(start@))]
    #[requires(Self::valid_rq_id(rq@))]
    pub fn iter_from(
        &self,
        start: ThreadId,
        rq: RunqueueId,
    ) -> RunQueueIter<'_, N_QUEUES, N_THREADS> {
        RunQueueIter {
            prev: start.0,
            rq_head: self.queues.peek_head(rq.0),
            // Clear higher priority runqueues.
            //bitcache: self.bitcache % (1 << (rq.0 + 1)),
            bitcache: self.clear_higher_priorities(self.bitcache, rq),
            queues: &self.queues,
        }
    }

    #[inline(always)]
    #[trusted]
    #[bitwise_proof]
    #[requires(bounded(rq@, 0, N_QUEUES@))]
    #[requires(valid_cache(bitcache, N_QUEUES))]
    #[ensures(valid_cache(result, N_QUEUES))]
    #[ensures(
        match self.head(rq) {
            Some(_) => result != 0usize,
            None    => true,
        }
    )]
    fn clear_higher_priorities(&self, bitcache: usize, rq: RunqueueId) -> usize {
        bitcache % (1 << (rq.0 + 1))
    }
}

#[inline(always)]
#[trusted]
#[ensures(forall<x: Int> 0 <= x && x < result@ - 1 ==> !val.nth_bit(x))]
#[ensures(val@ != 0 ==> val.nth_bit(result@))]
#[ensures(result@ <= USIZE_BITS@)]
fn leadz(val: usize) -> u32 {
    val.leading_zeros()
}

#[inline]
#[trusted]
#[bitwise_proof]
#[ensures(leadz.postcondition((val,), USIZE_BITS as u32 - result))]
#[ensures(forall<m: usize> valid_cache(val, m) ==> result < m as u32)]
#[ensures(val@ > 0 ==> result@ > 0)]
fn ffs(val: usize) -> u32 {
    USIZE_BITS as u32 - leadz(val)
}

#[logic]
fn valid_cache(bitcache: usize, n_queues: usize) -> bool {
    bitcache == 0usize || (bitcache >> (n_queues - 1usize)) == 0usize
}

#[logic]
pub fn bounded(value: Int, min: Int, max: Int) -> bool {
    pearlite! {
        min <= value && value < max
    }
}

/// Iterator over threads in a [`RunQueue`].
///
/// It starts from the highest priority queue and continues switching to lower
/// priority queues after circling through a queue once, until all queues
/// that are included in this iterator have been iterated.
pub struct RunQueueIter<'a, const N_QUEUES: usize, const N_THREADS: usize> {
    queues: &'a clist::CList<N_QUEUES, N_THREADS>,
    // Predecessor in the circular runqueue list.
    prev: u8,
    // Head of the currently iterated runqueue.
    rq_head: Option<u8>,
    // Bitcache with the remaining queues that have to be iterated.
    bitcache: usize,
}

impl<const N_QUEUES: usize, const N_THREADS: usize> IteratorSpec
    for RunQueueIter<'_, { N_QUEUES }, { N_THREADS }>
{
    #[logic]
    fn produces(self, _produced: Seq<Self::Item>, _end: Self) -> bool {
        true
    }

    #[logic]
    fn completed(&mut self) -> bool {
        true
    }

    #[logic(law)]
    fn produces_refl(self) {}

    #[logic(law)]
    #[requires(a.produces(ab, b))]
    #[requires(b.produces(bc, c))]
    #[ensures(a.produces(ab.concat(bc), c))]
    fn produces_trans(a: Self, ab: Seq<Self::Item>, b: Self, bc: Seq<Self::Item>, c: Self) {}
}

impl<const N_QUEUES: usize, const N_THREADS: usize> ::core::iter::Iterator
    for RunQueueIter<'_, { N_QUEUES }, { N_THREADS }>
{
    type Item = ThreadId;

    #[ensures(match result {
                    None => self.completed(),
                    Some(v) => (*self).produces(Seq::singleton(v), ^self)
                })]
    fn next(&mut self) -> Option<Self::Item> {
        let mut next = self.queues.peek_next(self.prev);
        if let Some(n) = self.rq_head
            && n == next
        {
            // Circled through whole queue, so switch to next one.
            let rq = ffs(self.bitcache) as u8 - 1;
            // Clear current runqueue from bitcache.
            //self.bitcache &= !(1 << rq);
            self.unset_bit_rq(rq);
            // Get head from remaining highest priority runqueue.
            self.rq_head = if self.bitcache > 0 {
                proof_assert!(valid_cache(self.bitcache, N_QUEUES));
                self.queues.peek_head(ffs(self.bitcache) as u8 - 1)
            } else {
                None
            };
            next = self.rq_head?;
        } else {
            return None;
        }
        self.prev = next;
        Some(ThreadId(next))
    }
}

impl<const N_QUEUES: usize, const N_THREADS: usize> Invariant
    for RunQueueIter<'_, N_QUEUES, N_THREADS>
{
    #[logic]
    fn invariant(self) -> bool {
        pearlite! {
            valid_cache(self.bitcache, N_QUEUES) &&
            bounded(self.prev@, 0, N_THREADS@) &&
            self.prev@ < clist::CList::<N_QUEUES, N_THREADS>::sentinel_log() &&
            match self.rq_head {
                Some(i) => self.bitcache != 0usize && bounded(i@, 0, N_THREADS@) && i@ < clist::CList::<N_QUEUES, N_THREADS>::sentinel_log(),
                    None    => true
            }
        }
    }
}

impl<const N_QUEUES: usize, const N_THREADS: usize> RunQueueIter<'_, N_QUEUES, N_THREADS> {
    #[inline(always)]
    #[trusted]
    #[bitwise_proof]
    #[requires(bounded(rq@, 0, N_QUEUES@))]
    fn unset_bit_rq(&mut self, rq: u8) {
        self.bitcache &= !(RunQueue::<N_QUEUES, N_THREADS>::mask_of_id(rq))
    }
}

mod clist {
    //! This module implements an array of `N_QUEUES` circular linked lists over an
    //! array of size `N_THREADS`.
    //!
    //! The array is used for "next" pointers, so each integer value in the array
    //! corresponds to one element, which can only be in one of the lists.

    use creusot_std::prelude::*;

    #[derive(Debug, Copy, std::clone::Clone)]
    pub struct CList<const N_QUEUES: usize, const N_THREADS: usize> {
        pub tail: [u8; N_QUEUES],
        pub next_idxs: [u8; N_THREADS],
    }

    impl<const N_QUEUES: usize, const N_THREADS: usize> Invariant for CList<N_QUEUES, N_THREADS> {
        #[logic(open)]
        fn invariant(self) -> bool {
            pearlite! {
                self.tail@.len() == N_QUEUES@
                    && self.next_idxs@.len() == N_THREADS@
                    && N_QUEUES@ < Self::sentinel_log()
                    && N_THREADS@ < Self::sentinel_log()
                    && Self::sentinel_log() <= 255
                    && forall<i: Int> 0 <= i && i < self.tail@.len() && self.tail@[i]@ != Self::sentinel_log() ==>
                        self.tail@[i]@ < N_THREADS@ //&& self.next_idxs@[self.tail@[i]@]@ != Self::sentinel_log()
                    && forall<i: Int> 0 <= i && i < self.next_idxs@.len() && self.next_idxs@[i]@ != Self::sentinel_log() ==>
                        self.next_idxs@[i]@ < N_THREADS@
            }
        }
    }

    #[cfg(not(creusot))]
    impl<const N_QUEUES: usize, const N_THREADS: usize> Default for CList<N_QUEUES, N_THREADS> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<const N_QUEUES: usize, const N_THREADS: usize> CList<N_QUEUES, N_THREADS> {
        #[logic(open(super))]
        pub fn valid_rq_id(id: Int) -> bool {
            pearlite! {
                super::bounded(id, 0, N_QUEUES@)
            }
        }

        #[logic(open(super))]
        pub fn valid_th_id(id: Int) -> bool {
            pearlite! {
                super::bounded(id, 0, N_THREADS@)
                    && id != Self::sentinel_log()
            }
        }

        #[logic(open(super))]
        pub fn valid_rq(&self, id: Int) -> bool {
            pearlite! {
                Self::valid_rq_id(id)
                    && self.tail@[id]@ != Self::sentinel_log()
            }
        }

        #[requires(N_QUEUES@ < 255)]
        #[requires(N_THREADS@ < 255)]
        pub const fn new() -> Self {
            // TODO: ensure N fits in u8
            // assert!(N<255); is not allowed in const because it could panic
            CList {
                tail: [Self::sentinel(); N_QUEUES],
                next_idxs: [Self::sentinel(); N_THREADS],
            }
        }

        #[ensures(result@ == Self::sentinel_log())]
        pub const fn sentinel() -> u8 {
            0xFF
        }

        #[logic]
        pub const fn sentinel_log() -> Int {
            pearlite! { 0xFF }
        }

        #[requires(Self::valid_rq_id(rq@))]
        #[ensures(result == !self.valid_rq(rq@))]
        #[ensures(result == (self.tail@[rq@]@ == Self::sentinel_log()))]
        pub fn is_empty(&self, rq: u8) -> bool {
            self.tail[rq as usize] == Self::sentinel()
        }

        #[expect(clippy::missing_panics_doc, reason = "internal")]
        #[requires(n@ < Self::sentinel_log())]
        #[requires(Self::valid_th_id(n@))]
        #[requires(self.valid_rq(rq@))]
        pub fn push(&mut self, n: u8, rq: u8) {
            assert!(n < Self::sentinel());
            if self.next_idxs[n as usize] != Self::sentinel() {
                return;
            }

            if let Some(head) = self.peek_head(rq) {
                // rq has an entry already, so
                // 1. n.next = old_tail.next ("first" in list)
                self.next_idxs[n as usize] = head;
                // 2. old_tail.next = n
                self.next_idxs[self.tail[rq as usize] as usize] = n;
                // 3. tail = n
                self.tail[rq as usize] = n;
            } else {
                // rq is empty, link both tail and n.next to n
                self.tail[rq as usize] = n;
                self.next_idxs[n as usize] = n;
            }
        }

        /// Removes a thread from the list.
        ///
        /// If the thread was the only thread in its runqueue, `Some` is returned
        /// with the ID of the now empty runqueue.
        #[requires(Self::valid_th_id(n@))]
        #[ensures(
            match result {
                Some(i) => Self::valid_rq_id(i@),
                None    => true
        }
        )]
        pub fn del(&mut self, n: u8) -> Option<u8> {
            if self.next_idxs[n as usize] == Self::sentinel() {
                return None;
            }
            let mut empty_runqueue = None;

            // Find previous thread in circular runqueue.
            let prev = position(&self.next_idxs, n)?;

            // Handle if thread is tail of a runqueue.
            if let Some(rq) = position(&self.tail, n) {
                if prev == n as usize {
                    // Runqueue is empty now.
                    self.tail[rq] = Self::sentinel();
                    empty_runqueue = Some(rq as u8);
                } else {
                    self.tail[rq] = prev as u8;
                }
            }

            self.next_idxs[prev] = self.next_idxs[n as usize];
            self.next_idxs[n as usize] = Self::sentinel();
            empty_runqueue
        }

        #[requires(Self::valid_rq_id(rq@))]
        #[ensures(result == self.head(rq))]
        pub fn pop_head(&mut self, rq: u8) -> Option<u8> {
            let head = self.peek_head(rq)?;
            // self.next_idxs[self.tail[rq as usize] as usize]
            if head == self.tail[rq as usize] {
                // rq's tail bites itself, so there's only one entry.
                // so, clear tail.
                self.tail[rq as usize] = Self::sentinel();
                // rq is now empty
            } else {
                // rq has multiple entries,
                // so set tail.next to head.next (second in list)
                self.next_idxs[self.tail[rq as usize] as usize] = self.next_idxs[head as usize];
            }

            // now clear head's next value
            self.next_idxs[head as usize] = Self::sentinel();
            Some(head)
        }

        #[logic(open)]
        pub fn head(self, rq: u8) -> Option<u8> {
            pearlite! {
                if self.tail[rq as usize]@ == Self::sentinel_log() {
                    None
                } else {
                    let res = self.next_idxs[self.tail[rq as usize] as usize];
                    if res@ == Self::sentinel_log() {
                        None
                    } else {
                        Some(res)
                    }
                    //Some(self.next_idxs[self.tail[rq as usize] as usize])
                }
            }
        }

        #[inline]
        #[requires(Self::valid_rq_id(rq@))]
        #[ensures(match result {
            Some(i) => Self::valid_th_id(i@),
            None    => true,
        })]
        #[ensures(result == self.head(rq))]
        pub fn peek_head(&self, rq: u8) -> Option<u8> {
            if self.is_empty(rq) {
                None
            } else {
                let res = self.next_idxs[self.tail[rq as usize] as usize];
                if res == Self::sentinel() {
                    None
                } else {
                    Some(res)
                }
                //Some(self.next_idxs[self.tail[rq as usize] as usize])
            }
        }

        #[requires(Self::valid_th_id(curr@))]
        pub fn peek_next(&self, curr: u8) -> u8 {
            self.next_idxs[curr as usize]
        }

        #[requires(self.valid_rq(rq@))]
        pub fn advance(&mut self, rq: u8) -> bool {
            let tail = self.tail[rq as usize];
            let head = self.next_idxs[tail as usize];
            if tail == head {
                // Catches the case that the runqueue only has a single element,
                // or is empty (in which case head == tail == Self::sentinel())
                return false;
            }

            self.tail[rq as usize] = head;
            true
        }
    }

    /// Helper function that is needed because hax doesn't support `Iterator::position` yet.
    #[ensures(match result {
        None =>
            forall<i: Int> 0 <= i && i < N@ ==> slice@[i]@ != search_item@,
        Some(i) =>
            super::bounded(i@, 0, N@) && slice@[i@]@ == search_item@
    })]
    fn position<const N: usize>(slice: &[u8; N], search_item: u8) -> Option<usize> {
        let mut i = 0;

        #[invariant(forall<j: Int> 0 <= j && j < i@ ==> slice@[j]@ != search_item@)]
        while i < N && slice[i] != search_item {
            i += 1;
        }
        if i < N { Some(i) } else { None }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_clist_basic() {
            let mut clist: CList<8, 32> = CList::new();
            assert!(clist.is_empty(0));
            clist.push(0, 0);
            assert_eq!(clist.pop_head(0), Some(0));
            assert_eq!(clist.pop_head(0), None);
        }

        #[test]
        fn test_clist_push_already_in_list() {
            let mut clist: CList<8, 32> = CList::new();
            assert!(clist.is_empty(0));
            clist.push(0, 0);
            clist.push(0, 0);
            assert_eq!(clist.pop_head(0), Some(0));
            assert_eq!(clist.pop_head(0), None);
            assert!(clist.is_empty(0));
        }

        #[test]
        fn test_clist_push_two() {
            let mut clist: CList<8, 32> = CList::new();
            assert!(clist.is_empty(0));
            clist.push(0, 0);
            clist.push(1, 0);
            assert_eq!(clist.pop_head(0), Some(0));
            assert_eq!(clist.pop_head(0), Some(1));
            assert_eq!(clist.pop_head(0), None);
            assert!(clist.is_empty(0));
        }

        #[test]
        fn test_clist_push_all() {
            const N: usize = 255;
            let mut clist: CList<8, N> = CList::new();
            assert!(clist.is_empty(0));
            for i in 0..(N - 1) {
                println!("pushing {}", i);
                clist.push(i as u8, 0);
            }
            for i in 0..(N - 1) {
                println!("{}", i);
                assert_eq!(clist.pop_head(0), Some(i as u8));
            }
            assert_eq!(clist.pop_head(0), None);
            assert!(clist.is_empty(0));
        }

        #[test]
        fn test_clist_advance() {
            let mut clist: CList<8, 32> = CList::new();
            assert!(clist.is_empty(0));
            clist.push(0, 0);
            clist.push(1, 0);
            clist.advance(0);
            assert_eq!(clist.pop_head(0), Some(1));
            assert_eq!(clist.pop_head(0), Some(0));
            assert_eq!(clist.pop_head(0), None);
            assert!(clist.is_empty(0));
        }

        #[test]
        fn test_clist_peek_head() {
            let mut clist: CList<8, 32> = CList::new();
            assert!(clist.is_empty(0));
            clist.push(0, 0);
            clist.push(1, 0);
            assert_eq!(clist.peek_head(0), Some(0));
            assert_eq!(clist.peek_head(0), Some(0));
            assert_eq!(clist.pop_head(0), Some(0));
            assert_eq!(clist.peek_head(0), Some(1));
            assert_eq!(clist.pop_head(0), Some(1));
            assert_eq!(clist.peek_head(0), None);
            assert_eq!(clist.peek_head(0), None);
            assert_eq!(clist.pop_head(0), None);
            assert!(clist.is_empty(0));
        }
    }
}
