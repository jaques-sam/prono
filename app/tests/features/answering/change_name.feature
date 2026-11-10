Feature: Changing my name
  As a participant whose name is refused
  I want to go back to the name field
  So that I can use another name

  Background:
    Given the Prono app is open

  Scenario: Going back shows the start screen with my name
    Given I am answering the survey as "Sam"
    When I go back to change my name
    Then I am on the start screen
    And the name field shows "Sam"

  Scenario: After a name that is taken I can use another name
    Given "Alice" already answered from another device, predicting month 3 of 2031
    And I am answering the survey as "Alice"
    When I submit my answers
    Then I see the error "This name is registered on another device"
    When I go back to change my name
    Then I am on the start screen
    And I see no error
    When I enter the name "Alice B"
    And I start the survey
    And I submit my answers
    Then I see the thanks to "Alice B"

  Scenario: The answers typed so far are not kept
    Given I am answering the survey as "Sam"
    And I set the year of "First man on Mars landing" to 2040
    When I go back to change my name
    And I start the survey
    Then the year of "First man on Mars landing" no longer shows 2040

  Scenario: Going back is not possible while waiting for the server
    Given I am answering the survey as "Sam"
    And the server does not respond
    When I submit my answers
    Then the "Change name" button is disabled
