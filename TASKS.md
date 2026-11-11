# Tasks

## Functional

1. [x] Show survey from file on "start survey" click
2. [x] Add answers for first user into db
3. [x] Show all results instead of the survey when a user who answered logs in again (see *)
4. [x] Get survey info from file survey
5. [x] Descriptions of survey & questions are not part of db surveys
6. [x] Do not allow duplicates in db
7. [x] Add ~~drop down list~~ DragValue for months & years and restrict number
8. [x] Show all answers
9.  [x] Time graph
10. [x] Implement ~~0MQ~~ REST server in ./backend
11. [x] Implement ~~0MQ~~ REST client
12. [x] Time graph of all user answers
13. [x] Restrict users for filling in (twice)
14. [x] Check when typing if user already exists/filld in survey, show warning
15. [x] Show errors over the UI
16. [ ] Add different user verification methods linking to the specific survey
  17. [x] a given pass phrase given by me: one pass phrase field in the app shows the surveys that have it, every survey needs one
  18. ~~[x] on given first + last name (list of names)~~ removed: anybody who knew a participant's name could answer for them
  18. [ ] is human (captcha?)
  19. [ ] login via
    20. [ ] Apple account
    21. [ ] Google account
    22. [ ] GitHub account
23. [x] Support a poll (bullet list) as answer type, where the results are shown with rectangles (longer means more votes). It adds "possible_answers" key in the questions list where "other" is always an (implicit) option which allows the respondent to specify a random possible answer.
24. [x] Make it possible to upload a new surveys, they should not be embedded in the binary any more, put them in a fixed location
25. [x] Make it possible to add scores to a survey, in that case all questions should have a correct answer
26. [x] Make an interactive prono-cli with navigation
27. [x] The admin user should be able to see all answers through the CLI tool, per user but also a table for all users

## Technical Dept
1. [x] Move prono-api to shared lib prono
2. [x] Make shared prono-db library
3. [x] Solve clippy pedantics, add `-Dclippy::pedantic`
4. [x] Make db thread async
5. [x] Add unit tests (especially for operations right before db)
6. [x] Add coverage checker
7. [x] Avoid asking survey responses on cursor movement etc.
8. [x] Use Uuid for question IDs directly
9. ~~[x] Use TLS for db traffic~~ --> db stays on server, app connects through REST only
10. [x] Use TLS for REST traffic
11. [x] Read config from default location(s) so app can run from any machine
12. [x] ~~Listen onblocked on recv from db thread~~ Update UI async (non-blocking HTTP client)
13. [x] Create backend service package for Synology
14. [x] Add a `UNIQUE (user_id, question_id)` constraint on `AnswerResponse` (`backend/install/schema.sql`)
