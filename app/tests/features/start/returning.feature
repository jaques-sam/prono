Feature: Coming back after answering
  As a participant who comes back
  I want to see everyone's answers right away
  So that I know my answers are final and can compare them with the others'

  Background:
    Given the Prono app is open

  Scenario: Starting the survey with my name again shows everyone's answers
    Given "Bob Brown" already answered from this device, predicting month 4 of 2033
    When I enter the name "Bob Brown"
    And I start the survey
    Then I see the thanks to "Bob Brown"
    And I see everyone's answers
    And each question's timeline shows 1 points
    And I cannot change my answers

  Scenario: My own answer stands out among the others on the timelines
    Given "Alice Adams" already answered from another device, predicting month 7 of 2036
    And "Bob Brown" already answered from this device, predicting month 4 of 2033
    When I enter the name "Bob Brown"
    And I start the survey
    Then each question's timeline shows 2 points
    And each question's timeline marks my own point with the date "04/2033"

  Scenario: Spaces around the name do not hide my earlier answers
    Given "Bob Brown" already answered from this device, predicting month 4 of 2033
    When I enter the first name "  Bob " and the last name " Brown "
    And I start the survey
    Then I see everyone's answers
    And I cannot change my answers

  Scenario: The answers of another name do not stop me from answering
    Given "Bob Brown" already answered from this device, predicting month 4 of 2033
    And "Alice Adams" already answered from another device, predicting month 7 of 2036
    When I enter the name "Sam Smith"
    And I start the survey
    Then I am answering the survey
    And the year of "First man on Mars landing" no longer shows 2033
    And I see no error

  Scenario: Answers that cannot be loaded do not stop me from answering
    Given I entered the name "Sam Smith"
    And I wait 1 seconds
    And the server cannot deliver the results
    When I start the survey
    Then I am answering the survey
    And I see no error
    And the "Submit" button is enabled

  Scenario: A server that does not respond does not stop me from answering
    Given I entered the name "Sam Smith"
    And I wait 1 seconds
    And the server does not respond
    When I start the survey
    Then I am answering the survey
    And the "Submit" button is enabled
