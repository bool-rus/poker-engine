/// Opaque hand score assigned by the external dealer process.
///
/// Higher values represent stronger poker hands. The engine only compares
/// scores numerically — it has no knowledge of card values, hand rankings,
/// or tiebreaker logic. All evaluation is delegated to the dealer.
///
/// # Examples
///
/// ```rust
/// use poker_engine::HandScore;
///
/// let score_a: HandScore = 8500000;
/// let score_b: HandScore = 4200000;
/// assert!(score_a > score_b);
/// ```
pub type HandScore = u64;
