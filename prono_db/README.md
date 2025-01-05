# Prono DB

### Setup

When the database is empty.

Login using following command.
Find the required parameters in [secure_config](../secure_config.toml)

```bash
mysql -h ${HOST} -P ${PORT} -u ${USER} -p -D db_prono
```

## Initialize expected SQL Tables

```sql
CREATE TABLE Users (
    user_id int not null,
    user_name text,
    email text
);

CREATE TABLE AnswerResponse (
    user text,
    question_id text,
    answer text
);
```

## Test

```sql
MariaDB [db_prono]> SELECT * FROM AnswerResponse;
+------+--------------------------------------+------------+
| user | question_id                          | answer     |
+------+--------------------------------------+------------+
| Sam  | 7873dd07-86a3-593b-ab8f-80bce8b7e84e | 00/08/2027 |
| Sam  | 68bcd727-1c0b-5c4b-8b56-515657894205 | 00/08/2029 |
| Sam  | 6ae332ad-b583-5748-97ff-65b13d86b42a | 00/12/2028 |
| Sam  | 84938a2e-9e40-562c-a68d-d33604ffac14 | 00/11/2035 |
+------+--------------------------------------+------------+
4 rows in set (0,040 sec)
```
