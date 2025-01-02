use super::Question;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Default))]
pub struct Survey {
    pub id: u64,
    pub description: String,
    pub questions: Vec<Question>,
}
