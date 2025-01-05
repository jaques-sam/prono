# Tasks

## Functional

- [x] Show survey from file on "start survey" click
- [x] Add answers for first user into db
- [ ] Add drop down list for months or restrict number
- [ ] Restrict users for filling in
- [ ] Refill survey when user logs in (see *)
- [ ] Show all answers
  - [ ] In table as output
  - [ ] In time graph
- [ ] ...


## Technical Dept
- [x] Move prono-api to shared lib prono
- [x] Make shared prono-db library
- [ ] Solve clippy pedantics, add `-Dclippy::pedantic`
- [ ] Add coverage checker
- [ ] Add unit tests (especially for operations right before db)
- [ ] (*) Avoid asking survey responses on cursor movement etc.
- [ ] Use Uuid for question IDs directly
- [ ] Use TLS for db traffic
- [ ] ...
