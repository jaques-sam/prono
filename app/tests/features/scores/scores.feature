Feature: Scores
  As a participant of a quiz
  I want to see how many answers everyone got right
  So that I know how well I did

  Background:
    Given the Prono app is open

  Scenario: The scores follow the submission, best first
    Given "Alice Adams" already got 2 answers of the quiz right from another device
    And "Bob Brown" already got 0 answers of the quiz right from another device
    And I answer the quiz as "Sam Smith"
    And I choose "Brussels" for "Capital of Belgium"
    And I choose "Brussels" for "Capital of France"
    When I submit my answers
    Then I see the score "1. Alice Adams: 2 of 2"
    And I see the score "2. Sam Smith: 1 of 2"
    And I see the score "3. Bob Brown: 0 of 2"
    And I see the score "2. Sam Smith: 1 of 2" below "1. Alice Adams: 2 of 2"
    And I see no error

  Scenario: The quiz does not show its correct answers while answering
    When I answer the quiz as "Sam Smith"
    Then I see the question "Capital of Belgium"
    And I see no scores

  Scenario: A survey without correct answers has no scores
    Given I am answering the survey as "Sam Smith"
    When I submit my answers
    Then I see everyone's answers
    And I see no scores

  Scenario: A perfect score
    Given I answer the quiz as "Sam Smith"
    And I choose "Brussels" for "Capital of Belgium"
    And I choose "Paris" for "Capital of France"
    When I submit my answers
    Then I see the score "1. Sam Smith: 2 of 2"
