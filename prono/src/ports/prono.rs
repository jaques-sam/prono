use crate::api::PronoApi;

pub trait Prono: PronoApi {
    fn survey(&self) -> crate::api::Survey;
}
