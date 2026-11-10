Feature: Refilling my earlier answers
  As a participant who comes back
  I want to see the answers I gave before
  So that I do not start from scratch

  Background:
    Given the Prono app is open

  Scenario: Starting the survey with my name again shows my earlier answers
    Given "Bob Brown" already answered from this device, predicting month 4 of 2033
    When I enter the name "Bob Brown"
    And I start the survey
    Then I am answering the survey
    And the month of "First man on Mars landing" shows 4
    And the year of "First man on Mars landing" shows 2033
    And the year of "First cargo only Moon landing" shows 2033

  Scenario: The earlier answers come from the same name only
    Given "Bob Brown" already answered from this device, predicting month 4 of 2033
    And "Alice Adams" already answered from another device, predicting month 7 of 2036
    When I enter the name "Alice Adams"
    And I start the survey
    Then the year of "First man on Mars landing" shows 2036

  Scenario: Spaces around the name do not hide my earlier answers
    Given "Bob Brown" already answered from this device, predicting month 4 of 2033
    When I enter the first name "  Bob " and the last name " Brown "
    And I start the survey
    Then the year of "First man on Mars landing" shows 2033

  Scenario: A new name starts with blank answers
    Given "Bob Brown" already answered from this device, predicting month 4 of 2033
    When I enter the name "Sam Smith"
    And I start the survey
    Then the year of "First man on Mars landing" no longer shows 2033
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
