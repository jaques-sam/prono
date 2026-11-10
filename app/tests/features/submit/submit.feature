Feature: Submitting answers
  As a participant
  I want to submit my answers
  So that they are stored and I see everyone's answers

  Background:
    Given the Prono app is open

  Scenario: Submitting stores my answers and shows the results
    Given I am answering the survey as "Sam Smith"
    When I submit my answers
    Then the server received the answers of "Sam Smith"
    And I see the thanks to "Sam Smith"
    And I see everyone's answers
    And I see no error

  Scenario: The name is trimmed when submitting
    Given I entered the first name "  Sam " and the last name " Smith  "
    And I start the survey
    When I submit my answers
    Then the server received the answers of "Sam Smith"

  Scenario: Submitting while waiting for the server is not possible
    Given I am answering the survey as "Sam Smith"
    And the server does not respond
    When I submit my answers
    Then the "Submit" button is disabled
    And the "Reset" button is disabled
    And the server received 1 submissions

  Scenario: A participant cannot answer the same survey twice
    Given I am answering the survey as "Sam Smith"
    And I submit my answers
    And I choose another survey
    When I start the survey
    Then I see everyone's answers
    And I cannot change my answers

  Scenario: A name used on another device is restricted
    Given "Alice Adams" already answered "C" to "What is your background" from another device
    And I am answering the survey as "Alice Adams"
    When I submit my answers
    Then I see the error "This name is registered on another device"
    And I am answering the survey

  Scenario: An unreachable server is reported over the survey
    Given I am answering the survey as "Sam Smith"
    And the server is unreachable
    When I submit my answers
    Then I see the error "Cannot reach the server"
    And I am answering the survey

  Scenario: The participant can try again after an error
    Given I am answering the survey as "Sam Smith"
    And the server is unreachable
    And I submit my answers
    And I dismiss the error
    And the server is back online
    When I submit my answers
    Then I see the thanks to "Sam Smith"
    And I see no error
