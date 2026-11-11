Feature: Surveys by access
  As a participant
  I want to see the surveys I may answer once the server knows who I am
  So that I am not offered surveys that are not for me

  Background:
    Given the Prono app is open

  Scenario: No surveys are offered before both names are typed
    Then I am asked to fill in my first and last name
    And I am offered no surveys
    When I enter the first name "Bob" and the last name ""
    And I wait 1 seconds
    Then I am asked to fill in my first and last name
    And I am offered no surveys
    And the server was not asked about any name

  Scenario: The surveys appear once both names are typed
    When I enter the name "Bob Brown"
    And I wait 1 seconds
    Then I am not asked to fill in my first and last name
    And I am offered the Starship survey

  Scenario: A survey with a name list is hidden from a name that is not on it
    When I enter the name "Alice Adams"
    And I wait 1 seconds
    Then I am offered the Starship survey
    And I am not offered the course survey

  Scenario: A survey with a name list is offered to a name on it
    Given "Bob Brown" is on the name list of the course survey
    When I enter the name "Bob Brown"
    And I wait 1 seconds
    Then I am offered the Starship survey
    And I am offered the course survey

  Scenario: The surveys follow the name
    Given "Bob Brown" is on the name list of the course survey
    And I entered the name "Bob Brown"
    And I wait 1 seconds
    When I enter the name "Alice Adams"
    Then I am offered no surveys
    When I wait 1 seconds
    Then I am not offered the course survey
    And I am offered the Starship survey

  Scenario: A listed name can start the course survey
    Given "Bob Brown" is on the name list of the course survey
    When I answer the course survey as "Bob Brown"
    Then I see the question "What is your background"

  Scenario: The surveys of a name that answered stay in the list after submitting
    Given I am answering the survey as "Sam Smith"
    And I submit my answers
    When I choose another survey
    Then I am offered the Starship survey
    And the server was asked only about "Sam Smith"

  Scenario: A lookup that fails is reported once and not repeated
    Given the server cannot deliver the results
    When I enter the name "Sam Smith"
    And I wait 1 seconds
    Then I see the error "Cannot reach the server"
    And I am offered no surveys
    When I dismiss the error
    And I wait 2 seconds
    Then I see no error
    And the server was asked only about "Sam Smith"

  Scenario: A failed lookup is tried again with another name
    Given the server cannot deliver the results
    And I entered the name "Sam Smith"
    And I wait 1 seconds
    And I dismiss the error
    And the server is back online
    When I enter the name "Sam Jones"
    And I wait 1 seconds
    Then I am offered the Starship survey
    And I see no error

  Scenario: A survey with a pass phrase asks for it with the surveys
    When I enter the name "Sam Smith"
    And I wait 1 seconds
    Then I am offered the Starship survey
    And I am asked for the pass phrase

  Scenario: The questions do not ask for the pass phrase again
    Given I am answering the survey as "Sam Smith"
    Then I am not asked for the pass phrase

  Scenario: The right pass phrase lets the answers in
    Given I am answering the survey as "Sam Smith"
    When I submit my answers
    Then I see the thanks to "Sam Smith"

  Scenario: A wrong pass phrase is refused
    Given I entered the name "Sam Smith"
    And I entered the pass phrase "wrong"
    And I start the survey
    When I submit my answers
    Then I see the error "Wrong pass phrase"
    And I am answering the survey

  Scenario: A survey cannot be answered without its pass phrase
    Given I entered the name "Sam Smith"
    And I entered the pass phrase ""
    And I start the survey
    When I submit my answers
    Then I see the error "Wrong pass phrase"
