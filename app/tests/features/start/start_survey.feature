Feature: Starting a survey
  As a participant
  I want to enter my name and start a survey
  So that I can fill it in

  Background:
    Given the Prono app is open

  Scenario: The start screen offers the surveys
    Then I am on the start screen

  Scenario: Starting a survey with a name
    When I enter the name "Sam"
    And I start the survey
    Then I am answering the survey
    And I see no error

  Scenario: Spaces around the name are ignored
    When I enter the name "  Sam  "
    And I start the survey
    Then I am answering the survey

  Scenario: A survey cannot be started without a name
    When I start the survey
    Then I see the error "Please fill in your name"
    And I am on the start screen

  Scenario: A name of only spaces is refused
    When I enter the name "   "
    And I start the survey
    Then I see the error "Please fill in your name"
    And I am on the start screen

  Scenario: A name that is too long is refused
    When I enter a name of 65 characters
    And I start the survey
    Then I see the error "Please fill in your name (at most 64 characters)"
    And I am on the start screen

  Scenario: The longest name is accepted
    When I enter a name of 64 characters
    And I start the survey
    Then I am answering the survey

  Scenario: An error goes away when dismissed
    When I start the survey
    And I dismiss the error
    Then I see no error

  Scenario: Starting with a valid name clears an earlier error
    Given I start the survey
    And I entered the name "Sam"
    When I start the survey
    Then I see no error
    And I am answering the survey
