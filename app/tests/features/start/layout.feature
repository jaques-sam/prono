Feature: Layout of the start form
  As a participant on a small screen
  I want the form to keep its shape and scroll sideways
  So that I can reach everything without reading squashed fields

  Background:
    Given the Prono app is open

  Scenario: The submit button ends where the name fields end
    Then the Submit button ends where the name fields end

  Scenario: The submit button keeps lining up in a narrow window
    Given the window is 300 points wide
    Then the Submit button ends where the name fields end

  Scenario: A narrow window does not squash the fields
    Given the window is 300 points wide
    Then the name fields keep their full width

  Scenario: A narrow window scrolls sideways
    Given the window is 300 points wide
    When I scroll right by 100 points
    Then the form moved left by 100 points

  Scenario: A wide window does not scroll sideways
    When I scroll right by 100 points
    Then the form moved left by 0 points
