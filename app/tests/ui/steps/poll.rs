use cucumber::{given, then, when};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Node;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use prono_api::Answer;

use crate::fake_backend::MY_DEVICE;
use crate::steps::common::{COURSE_ID, course};
use crate::world::World;

/// The label of the radio button for an answer of one's own.
const OTHER: &str = "Other";

impl World {
    /// The radio button `label` of the poll question `question`.
    fn poll_radio(&self, question: &str, label: &str) -> Node<'_> {
        // The radio buttons of all the polls follow each other: the options
        // of each question, then its "Other".
        let mut before = 0;
        for survey_question in &course().questions {
            if survey_question.text == question {
                let options = &survey_question.possible_answers;
                let position = options.iter().position(|option| option == label);
                let index = before
                    + position.unwrap_or_else(|| {
                        assert_eq!(label, OTHER, "no option \"{label}\" in \"{question}\"");
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
        panic!("no question \"{question}\" in the course survey");
    }

    fn answer_to(&self, question: &str) -> Answer {
        let index = course()
            .questions
            .iter()
            .position(|q| q.text == question)
            .unwrap_or_else(|| panic!("no question \"{question}\""));
        match &self.session().state.phase {
            prono_app::state::Phase::Answering { answers, .. } => answers[index].clone(),
            _ => panic!("not answering a survey"),
        }
    }
}

#[given(expr = "I answer the course survey as {string}")]
#[when(expr = "I answer the course survey as {string}")]
fn answer_course(world: &mut World, name: String) {
    world.answer_survey_as(&name, &course().description);
}

#[given(expr = "{string} already answered {string} to {string} from this device")]
fn answered_here(world: &mut World, name: String, answer: String, question: String) {
    seed(world, &name, MY_DEVICE, &answer, &question);
}

#[given(expr = "{string} already answered {string} to {string} from another device")]
fn answered_elsewhere(world: &mut World, name: String, answer: String, question: String) {
    seed(world, &name, &format!("device-of-{name}"), &answer, &question);
}

/// `name` answered `question` with `answer`, and the other questions with
/// their first option.
fn seed(world: &World, name: &str, device: &str, answer: &str, question: &str) {
    world.session().backend.seed_answers(COURSE_ID, name, device, |asked| {
        let chosen = if asked.text == question {
            answer
        } else {
            &asked.possible_answers[0]
        };
        Answer::Poll(chosen.to_string())
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

#[given(expr = "I type the answer {string}")]
#[when(expr = "I type the answer {string}")]
fn type_answer(world: &mut World, text: String) {
    world.harness.get_by_role(Role::TextInput).click();
    world.settle();
    world.harness.event(egui::Event::Text(text));
    world.settle();
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
