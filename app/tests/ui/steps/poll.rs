use cucumber::{given, then, when};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Node;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use prono_api::{Answer, samples};

use crate::fake_backend::MY_DEVICE;
use crate::fake_backend::{COURSE_ID, COURSE_PASS_PHRASE};
use crate::steps::common::course;
use crate::world::World;

impl World {
    /// The radio button `label` of the poll question `question` of the
    /// survey being answered.
    fn poll_radio(&self, question: &str, label: &str) -> Node<'_> {
        let survey = self.session().state.survey().expect("a survey");
        // The radio buttons of all the polls follow each other: the options
        // of each question, then its other label.
        let mut before = 0;
        for survey_question in &survey.questions {
            if survey_question.text == question {
                let options = &survey_question.possible_answers;
                let position = options.iter().position(|option| option == label);
                let index = before
                    + position.unwrap_or_else(|| {
                        assert_eq!(
                            label,
                            survey_question.other_label(),
                            "no option \"{label}\" in \"{question}\""
                        );
                        options.len()
                    });
                return self
                    .harness
                    .query_all_by_role(Role::RadioButton)
                    .nth(index)
                    .unwrap_or_else(|| panic!("no radio button \"{label}\" for \"{question}\""));
            }
            before += survey_question.possible_answers.len() + 1;
        }
        panic!("no question \"{question}\" in the survey");
    }

    fn answer_to(&self, question: &str) -> Answer {
        match &self.session().state.phase {
            prono_app::state::Phase::Answering { survey, answers } => {
                let index = survey
                    .questions
                    .iter()
                    .position(|q| q.text == question)
                    .unwrap_or_else(|| panic!("no question \"{question}\""));
                answers[index].clone()
            }
            _ => panic!("not answering a survey"),
        }
    }
}

#[given(expr = "I answer the course survey as {string}")]
#[when(expr = "I answer the course survey as {string}")]
fn answer_course(world: &mut World, name: String) {
    world.answer_survey_as(&name, &course().description, COURSE_PASS_PHRASE);
}

#[when("I start the course survey")]
fn start_course(world: &mut World) {
    world.start_survey(&course().description, COURSE_PASS_PHRASE);
}

#[given(expr = "{string} already answered {string} to {string} from this device")]
fn answered_here(world: &mut World, name: String, answer: String, question: String) {
    seed(world, &name, MY_DEVICE, &answer, &question);
}

#[given(expr = "{string} already answered {string} to {string} from another device")]
fn answered_elsewhere(world: &mut World, name: String, answer: String, question: String) {
    seed(world, &name, &format!("device-of-{name}"), &answer, &question);
}

/// `name` answered the poll `question` with `answer`, and the other questions
/// with the sample answer.
fn seed(world: &World, name: &str, device: &str, answer: &str, question: &str) {
    world.session().backend.seed_answers(COURSE_ID, name, device, |asked| {
        if asked.text == question {
            Answer::Poll(answer.to_string())
        } else {
            samples::answer_to(asked)
        }
    });
}

#[then(expr = "I see the option {string} of {string}")]
fn sees_option(world: &mut World, option: String, question: String) {
    world.poll_radio(&question, &option);
}

#[given(expr = "I choose {string} for {string}")]
#[when(expr = "I choose {string} for {string}")]
fn choose(world: &mut World, option: String, question: String) {
    world.poll_radio(&question, &option).click();
    world.settle();
}

/// Types `text` into the field with the placeholder `hint`: the field of an
/// answer of one's own, or of a text question.
fn type_into_hinted(world: &mut World, hint: &str, text: String) {
    world
        .harness
        .query_all_by_role(Role::TextInput)
        .find(|field| field.accesskit_node().placeholder() == Some(hint))
        .unwrap_or_else(|| panic!("no field with the hint \"{hint}\""))
        .click();
    world.settle();
    world.harness.event(egui::Event::Text(text));
    world.settle();
}

#[given(expr = "I type the answer {string}")]
#[when(expr = "I type the answer {string}")]
fn type_answer(world: &mut World, text: String) {
    type_into_hinted(world, "your own answer", text);
}

#[given(expr = "I type the text answer {string}")]
#[when(expr = "I type the text answer {string}")]
fn type_text_answer(world: &mut World, text: String) {
    type_into_hinted(world, "your answer here", text);
}

#[then(expr = "the answer to {string} is {string}")]
fn answer_is(world: &mut World, question: String, expected: String) {
    assert_eq!(world.answer_to(&question), Answer::Poll(expected));
}

#[then(expr = "the option {string} of {string} is selected")]
fn option_selected(world: &mut World, option: String, question: String) {
    assert_eq!(
        world.poll_radio(&question, &option).accesskit_node().toggled(),
        Some(Toggled::True)
    );
}

#[then(expr = "the option {string} of {string} is not selected")]
fn option_not_selected(world: &mut World, option: String, question: String) {
    assert_ne!(
        world.poll_radio(&question, &option).accesskit_node().toggled(),
        Some(Toggled::True)
    );
}

#[then(expr = "the poll results show {string}")]
fn poll_results_show(world: &mut World, row: String) {
    assert!(world.shows(&row), "no chart row \"{row}\"");
}
