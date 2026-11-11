Feature: Warning about a name that already answered
  As a participant
  I want to be warned while typing my name if it already filled in a survey
  So that I notice before I start

  Background:
    Given the Prono app is open

  Scenario: A name that already answered is warned about once typing pauses
    Given "Alice Adams" already answered from this device
    When I enter the name "Alice Adams"
    And I wait 1 seconds
    Then I see the warning that "Alice Adams" already filled in a survey

  Scenario: A fresh name gets no warning
    Given "Alice Adams" already answered from this device
    When I enter the name "Bob Brown"
    And I wait 1 seconds
    Then I see no name warning
    And the server was asked only about "Bob Brown"

  Scenario: The warning goes away when the name is changed to a fresh one
    Given "Alice Adams" already answered from this device
    And I entered the name "Alice Adams"
    And I wait 1 seconds
    When I enter the name "Carol Clark"
    Then I see no name warning
    When I wait 1 seconds
    Then I see no name warning

  Scenario: The backend is not asked while the name keeps changing
    Given "Alice Adams" already answered from this device
    When I enter the name "A Ng"
    And I wait 0.2 seconds
    And I enter the name "Al Ng"
    And I wait 0.2 seconds
    And I enter the name "Ali Ng"
    And I wait 0.2 seconds
    Then the server was not asked about any name
    And I see no name warning
    When I wait 1 seconds
    Then the server was asked only about "Ali Ng"

  Scenario: A lookup that fails is reported once and not repeated
    Given the server cannot deliver the results
    When I enter the name "Sam Smith"
    And I wait 1 seconds
    Then I see the error "Cannot reach the server"
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
    Then the server was asked about "Sam Smith" and then about "Sam Jones"
    And I see no error
