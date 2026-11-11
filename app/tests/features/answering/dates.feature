Feature: Predicting dates
  As a participant
  I want to enter a month and a year for each question
  So that I can predict when it happens, within sensible limits

  Background:
    Given the Prono app is open
    And I am answering the survey as "Sam Smith"

  Scenario: Every question asks for a date
    Then I see the question "First cargo only Moon landing"
    And I see the question "First man on the Moon landing"
    And I see the question "First cargo only Mars landing"
    And I see the question "First man on Mars landing"

  Scenario: A valid date is accepted
    When I set the month of "First man on Mars landing" to 3
    And I set the year of "First man on Mars landing" to 2031
    Then the month of "First man on Mars landing" shows 3
    And the year of "First man on Mars landing" shows 2031

  Scenario: Dates are entered per question
    When I set the year of "First man on Mars landing" to 2040
    Then the year of "First man on Mars landing" shows 2040
    And the year of "First cargo only Moon landing" no longer shows 2040

  Scenario Outline: The month is limited to 1 through 12
    When I set the month of "First cargo only Moon landing" to <typed>
    Then the month of "First cargo only Moon landing" shows <shown>

    Examples:
      | typed | shown |
      | 0     | 1     |
      | 1     | 1     |
      | 12    | 12    |
      | 13    | 12    |
      | 99    | 12    |

  Scenario Outline: The year is limited to 2024 through 2100
    When I set the year of "First cargo only Moon landing" to <typed>
    Then the year of "First cargo only Moon landing" shows <shown>

    Examples:
      | typed | shown |
      | 1999  | 2024  |
      | 2024  | 2024  |
      | 2100  | 2100  |
      | 2101  | 2100  |
      | 3000  | 2100  |

  @debug-only
  Scenario: Resetting restores the starting dates
    Given I set the year of "First cargo only Moon landing" to 2077
    When I reset my answers
    Then the year of "First cargo only Moon landing" no longer shows 2077

  @release-only
  Scenario: A filled-in prono cannot be reset
    Then there is no "Reset" button

  Scenario: The timeline of my predictions follows my dates
    Then the timeline shows 4 points
    When I set the month of "First cargo only Moon landing" to 3
    And I set the year of "First cargo only Moon landing" to 2031
    And I hover over the latest point of the timeline
    Then the timeline shows 4 points
    And the tooltip shows the date "03/2031"
