# Tasks

## Functional

1. [x] Show survey from file on "start survey" click
2. [x] Add answers for first user into db
3. [ ] ~~Refill survey when user logs in (see *)~~
4. [ ] Get survey info from file survey
5. [ ] Descriptions of survey & questions are not part of db surveys -> busy
6. [x] Do not allow duplicates in db
7. [ ] Add drop down list for months or restrict number
8. [ ] Restrict users for filling in
9. [x] Show all answers
10. [ ] In table as output
11. [x] In time graph
12. [ ] Implement MQTT server in ./server
13. [ ] Implement MQTT client
14. [ ] In time graph of all user answers


## Technical Dept
1. [x] Move prono-api to shared lib prono
2. [x] Make shared prono-db library
3. [x] Solve clippy pedantics, add `-Dclippy::pedantic`
4. [x] Make db thread async
5. [ ] Add unit tests (especially for operations right before db)
6. [ ] Add coverage checker
7. [ ] (*) Avoid asking survey responses on cursor movement etc.
8. [ ] Use Uuid for question IDs directly
9. [ ] Use TLS for db traffic
10. [ ] Read config from default location(s) so app can run from any machine
11. [ ] Listen onblocked on recv from db thread, update UI async
12. [ ] ...
