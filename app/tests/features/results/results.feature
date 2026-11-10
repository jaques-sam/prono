Feature: Everyone's answers
  As a participant
  I want to see all answers on a time graph
  So that I can compare my predictions with the others'

  Background:
    Given the Prono app is open

  Scenario: The results graph shows every participant for each question
    Given "Alice Adams" already answered from another device, predicting month 3 of 2031
    And "Bob Brown" already answered from another device, predicting month 6 of 2028
    And I am answering the survey as "Sam Smith"
    When I submit my answers
    Then each question's timeline shows 3 points

  Scenario: A point names who predicted the date
    Given "Alice Adams" already answered from another device, predicting month 3 of 2031
    And I am answering the survey as "Sam Smith"
    And I submit my answers
    When I hover over the latest point of the timeline
    Then the tooltip shows the date "03/2031" of "Alice Adams"

  Scenario: Choosing another survey returns to the start screen
    Given I am answering the survey as "Sam Smith"
    And I submit my answers
    When I choose another survey
    Then I am on the start screen

  Scenario: Results that cannot be loaded are reported over the UI
    Given I am answering the survey as "Sam Smith"
    And the server cannot deliver the results
    When I submit my answers
    Then I see the thanks to "Sam Smith"
    And I see the error "Cannot reach the server"

  Scenario: The results are fetched again once the error is dismissed
    Given I am answering the survey as "Sam Smith"
    And the server cannot deliver the results
    And I submit my answers
    And the server is back online
    When I dismiss the error
    Then I see no error
    And each question's timeline shows 1 points
