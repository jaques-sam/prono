pub trait ApiSurvey{}

pub trait PronoApi<S> {
    fn survey(&self) -> S;
}
