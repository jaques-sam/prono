Feature: Bigger input field
  As a participant
  I want a bigger input field for a longer answer
  So that I can write more than a few words and still see all of it

  Scenario: A multiline question gets a taller field than a single-line one
    When I answer the essay survey as "Sam Smith"
    Then the multiline answer field is taller than the single-line one

  Scenario: A long answer wraps and makes the field grow instead of scrolling sideways
    Given I answer the essay survey as "Sam Smith"
    And I note the height of the multiline answer field
    When I type 200 words into the multiline answer
    Then the multiline answer field is taller than noted
