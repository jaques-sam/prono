Feature: Poll questions
  As a participant
  I want to pick one of the offered options, or give an answer of my own
  So that the poll captures what I think

  Background:
    Given the Prono app is open

  Scenario: Every option is offered next to "Other"
    When I answer the course survey as "Sam Smith"
    Then I see the option "C" of "What is your background"
    And I see the option "C++" of "What is your background"
    And I see the option "Other" of "What is your background"

  Scenario: A poll can name the answer of one's own
    When I answer the quiz as "Sam Smith"
    Then I see the option "Another city:" of "Capital of France"
    And I see the option "Other" of "Capital of Belgium"

  Scenario: A named answer of one's own lets the participant type
    Given I answer the quiz as "Sam Smith"
    When I choose "Another city:" for "Capital of France"
    And I type the answer "Lyon"
    Then the answer to "Capital of France" is "Lyon"
    And the option "Another city:" of "Capital of France" is selected

  Scenario: Choosing an option gives that answer
    Given I answer the course survey as "Sam Smith"
    When I choose "C++" for "What is your background"
    Then the answer to "What is your background" is "C++"
    And the option "C++" of "What is your background" is selected

  Scenario: Another option replaces the choice
    Given I answer the course survey as "Sam Smith"
    And I choose "C++" for "What is your background"
    When I choose "C" for "What is your background"
    Then the answer to "What is your background" is "C"
    And the option "C++" of "What is your background" is not selected

  Scenario: "Other" lets the participant type an answer
    Given I answer the course survey as "Sam Smith"
    When I choose "Other" for "What is your background"
    And I type the answer "Rust"
    Then the answer to "What is your background" is "Rust"
    And the option "Other" of "What is your background" is selected

  Scenario: "Other" stays selected until something is typed
    Given I answer the course survey as "Sam Smith"
    When I choose "Other" for "What is your background"
    Then the option "Other" of "What is your background" is selected
    And the answer to "What is your background" is ""

  Scenario: Going back to an option hides the typed answer
    Given I answer the course survey as "Sam Smith"
    And I choose "Other" for "What is your background"
    And I type the answer "Rust"
    When I choose "C" for "What is your background"
    Then the answer to "What is your background" is "C"
    And the option "Other" of "What is your background" is not selected

  Scenario: An answer of one's own shows in the results after logging in again
    Given "Sam Smith" already answered "Rust" to "What is your background" from this device
    When I enter the name "Sam Smith"
    And I start the course survey
    Then the poll results show "Rust (other): 1 vote (100%)"
    And I cannot change my answers

  @debug-only
  Scenario: Resetting the answers clears "Other"
    Given I answer the course survey as "Sam Smith"
    And I choose "Other" for "What is your background"
    And I type the answer "Rust"
    When I reset my answers
    Then the answer to "What is your background" is ""
    And the option "Other" of "What is your background" is not selected

  Scenario: The results show a bar for every option, also one without votes
    Given I answer the course survey as "Sam Smith"
    And I choose "C" for "What is your background"
    And I choose "Of course!" for "I come to the course!"
    And I choose "no" for "Do you have any diatery requirements or allergies?"
    When I submit my answers
    Then the poll results show "C: 1 vote (100%)"
    And the poll results show "C++: 0 votes (0%)"

  Scenario: The course survey shows only my own answers, not those of the others
    Given "Alice Adams" already answered "C" to "What is your background" from another device
    And "Bob Brown" already answered "Rust" to "What is your background" from another device
    And I answer the course survey as "Sam Smith"
    And I choose "C++" for "What is your background"
    And I choose "Of course!" for "I come to the course!"
    And I choose "no" for "Do you have any diatery requirements or allergies?"
    When I submit my answers
    Then I see only my own answers
    And the poll results show "C++: 1 vote (100%)"
    And the poll results show "C: 0 votes (0%)"

  Scenario: Another device does not see the answers of a participant
    Given "Sam Smith" already answered "C" to "What is your background" from another device
    When I enter the name "Sam Smith"
    And I start the course survey
    Then I see the question "What is your background"
    And the "Submit" button is enabled
