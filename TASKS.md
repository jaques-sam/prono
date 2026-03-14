# Tasks

## Functional

1. [x] Show survey from file on "start survey" click
2. [x] Add answers for first user into db
3. [x] Refill survey when user logs in (see *)
4. [ ] Get survey info from file survey
5. [ ] Descriptions of survey & questions are not part of db surveys -> busy
6. [x] Do not allow duplicates in db
7. [ ] Add drop down list for months or restrict number
8. [x] Show all answers
9.  [x] Time graph
10. [x] Implement ~~0MQ~~ REST server in ./backend
11. [x] Implement ~~0MQ~~ REST client
12. [x] Time graph of all user answers
13. [x] Restrict users for filling in (twice)
14. [ ] Check when typing if user already exists/filld in survey, show warning
15. [ ] Show errors over the UI
16. [ ] Add user verification (pass phrase?/is human?)
17. [ ] Table as output

## Technical Dept
1. [x] Move prono-api to shared lib prono
2. [x] Make shared prono-db library
3. [x] Solve clippy pedantics, add `-Dclippy::pedantic`
4. [x] Make db thread async
5. [x] Add unit tests (especially for operations right before db)
6. [x] Add coverage checker
7. [ ] Avoid asking survey responses on cursor movement etc.
8. [ ] Use Uuid for question IDs directly
9. [ ] Use TLS for db traffic
10. [ ] Use TLS for REST traffic
11. [x] Read config from default location(s) so app can run from any machine
12. [ ] Listen onblocked on recv from db thread, update UI async
13. [x] Create backend service package for Synology
14. [ ] Deploy backend automatically on release update [hard to impossible]
