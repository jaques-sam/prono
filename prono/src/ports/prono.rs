use crate::api::Surveys;

pub trait Prono: Surveys {
    fn survey(&self) -> crate::api::Survey;
}
