Feature: Surveys by pass phrase
  As a participant
  I want to see only the surveys my pass phrase opens
  So that surveys that are not for me stay hidden

  Background:
    Given the Prono app is open

  Scenario: No surveys are offered before a pass phrase is submitted
    Then I am asked for my pass phrase
    And I am offered no surveys
    And the server was not asked for any surveys

  Scenario: Typing the pass phrase is not submitting it
    When I enter the pass phrase "open sesame"
    And I wait 1 seconds
    Then I am offered no surveys
    And the server was not asked for any surveys

  Scenario: The right pass phrase shows the surveys that have it
    When I enter the pass phrase "open sesame"
    And I submit the pass phrase
    Then I am offered the Starship survey
    And I am offered the quiz survey
    And I am not offered the course survey
    And I am not asked for my pass phrase
    And I see no error

  Scenario: Another pass phrase shows other surveys
    When I enter the pass phrase "rust rocks"
    And I submit the pass phrase
    Then I am offered the course survey
    And I am not offered the Starship survey
    And I am not offered the quiz survey

  Scenario: Surrounding spaces of the pass phrase are ignored
    When I enter the pass phrase "  open sesame "
    And I submit the pass phrase
    Then I am offered the Starship survey

  Scenario: A wrong pass phrase shows nothing
    When I enter the pass phrase "wrong"
    And I submit the pass phrase
    Then I see the error "Wrong pass phrase"
    And I am offered no surveys

  Scenario: A pass phrase has to match exactly
    When I enter the pass phrase "Open Sesame"
    And I submit the pass phrase
    Then I see the error "Wrong pass phrase"
    And I am offered no surveys

  Scenario: A blank pass phrase shows nothing
    When I enter the pass phrase ""
    And I submit the pass phrase
    Then I see the error "Wrong pass phrase"
    And I am offered no surveys

  Scenario: Changing the pass phrase hides the surveys until it is submitted again
    Given I entered the pass phrase "open sesame"
    And I submitted the pass phrase
    When I enter the pass phrase "rust rocks"
    Then I am offered no surveys
    And I am asked for my pass phrase
    When I submit the pass phrase
    Then I am offered the course survey
    And I am not offered the Starship survey

  Scenario: A wrong pass phrase takes back the surveys of an earlier one
    Given I entered the pass phrase "open sesame"
    And I submitted the pass phrase
    When I enter the pass phrase "wrong"
    And I submit the pass phrase
    Then I see the error "Wrong pass phrase"
    And I am offered no surveys
    When I dismiss the error
    And I enter the pass phrase "open sesame"
    Then I am asked for my pass phrase

  Scenario: One pass phrase field, whatever the surveys
    When I enter the pass phrase "open sesame"
    And I submit the pass phrase
    Then there is exactly one pass phrase field
    And I am offered the Starship survey
    And I am offered the quiz survey

  Scenario: Another survey can be answered with its own pass phrase
    When I answer the course survey as "Sam Smith"
    Then I see the question "What is your background"

  Scenario: A survey cannot be started without a name
    Given I entered the pass phrase "open sesame"
    And I submitted the pass phrase
    When I click the Starship survey
    Then I see the error "Please fill in your first and last name"
    And I am on the start screen

  Scenario: The questions do not ask for the pass phrase again
    Given I am answering the survey as "Sam Smith"
    Then there is no pass phrase field

  Scenario: The right pass phrase lets the answers in
    Given I am answering the survey as "Sam Smith"
    When I submit my answers
    Then I see the thanks to "Sam Smith"

  Scenario: A pass phrase that stopped being right refuses the answers
    Given I am answering the survey as "Sam Smith"
    And the pass phrase of the Starship survey was changed on the server
    When I submit my answers
    Then I see the error "Wrong pass phrase"
    And I am answering the survey

  Scenario: The surveys stay in the list after submitting
    Given I am answering the survey as "Sam Smith"
    And I submit my answers
    When I choose another survey
    Then I am offered the Starship survey
    And the server was asked for surveys 1 time
    And the server was asked only about "Sam Smith"
