Feature: Starting a survey
  As a participant
  I want to enter my name and start a survey
  So that I can fill it in

  Background:
    Given the Prono app is open

  Scenario: Without a name I am asked for one instead of offered surveys
    Then I am asked to fill in my first and last name
    And I am offered no surveys

  Scenario: Starting a survey with a first and a last name
    When I enter the name "Sam Smith"
    And I start the survey
    Then I am answering the survey
    And I see no error

  Scenario: Spaces around the names are ignored
    When I enter the first name "  Sam  " and the last name "  Smith "
    And I start the survey
    Then I am answering the survey

  Scenario: A first name alone is not enough
    When I enter the first name "Sam" and the last name ""
    And I wait 1 seconds
    Then I am offered no surveys

  Scenario: A last name alone is not enough
    When I enter the first name "" and the last name "Smith"
    And I wait 1 seconds
    Then I am offered no surveys

  Scenario: Names of only spaces are refused
    When I enter the first name "   " and the last name "   "
    And I wait 1 seconds
    Then I am asked to fill in my first and last name
    And I am offered no surveys

  Scenario: A name that is too long is not accepted
    When I enter a first name of 63 characters and the last name "b"
    And I wait 1 seconds
    Then I am asked to fill in my first and last name
    And I am offered no surveys

  Scenario: The longest name is accepted
    When I enter a first name of 62 characters and the last name "b"
    And I start the survey
    Then I am answering the survey

  Scenario: An error goes away when dismissed
    Given I entered the name "Sam Smith"
    And I wait 1 seconds
    And the server cannot deliver the results
    And I entered the name "Sam Jones"
    And I wait 1 seconds
    When I dismiss the error
    Then I see no error
