Feature: Warning about a name that already answered
  As a participant
  I want to be warned while typing my name if it already filled in a survey
  So that I notice before I start

  Background:
    Given the Prono app is open

  Scenario: A name that already answered is warned about once typing pauses
    Given "Alice" already answered from this device
    When I enter the name "Alice"
    And I wait 1 seconds
    Then I see the warning that "Alice" already filled in a survey

  Scenario: A fresh name gets no warning
    Given "Alice" already answered from this device
    When I enter the name "Bob"
    And I wait 1 seconds
    Then I see no name warning
    And the server was asked only about "Bob"

  Scenario: The warning goes away when the name is changed to a fresh one
    Given "Alice" already answered from this device
    And I entered the name "Alice"
    And I wait 1 seconds
    When I enter the name "Carol"
    Then I see no name warning
    When I wait 1 seconds
    Then I see no name warning

  Scenario: The backend is not asked while the name keeps changing
    Given "Alice" already answered from this device
    When I enter the name "A"
    And I wait 0.2 seconds
    And I enter the name "Al"
    And I wait 0.2 seconds
    And I enter the name "Ali"
    And I wait 0.2 seconds
    Then the server was not asked about any name
    And I see no name warning
    When I wait 1 seconds
    Then the server was asked only about "Ali"
