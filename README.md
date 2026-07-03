# saba

saba is database of CSV files management.

## Build and run

```bash
$ # create envrionment directory for database
$ mkdir my_env

$ # run with environment directory
$ cargo run my_env
```

## Syntax and future

### CREATE

Create database (create directory).

```
CREATE DATABASE my_db;
```

Create table (create CSV file).

```
CREATE TABLE my_table (
	id: I64 PRIMARY_KEY AUTO_INCREMENT,
	weight: F64,
	name: CHAR[100],
);
```

## DROP

Drop database (remove directory).

```
DROP DATABASE my_db;
```

Drop table (remove CSV file).

```
DROP TABLE my_table;
```

## USE

Select current using database.

```
USE my_db
```

## DESC

Show table info.

```
DESC my_table;
```

## ALTER (change table)

Add column into table.

```
ALTER TABLE my_table ADD COLUMN name CHAR[100];
```

Drop column from table.

```
ALTER TABLE my_table DROP COLUMN name;
```

## GET (get CSV records)

```
GET id, weight, name OF my_table;
GET id, weight, name OF my_table WHERE id == 2;
GET ALL id, weight, name OF my_table WHERE id < 5;
```

## ADD (add CSV record)

```
ADD id = 1, weight = 60.3, name = "hige" OF my_table;
```

## DEL (delete CSV records)

```
DEL OF my_table WHERE id == 2;
DEL ALL OF my_table;
DEL ALL OF my_table WHERE id < 5;
```

## SET (update CSV records)

```
SET weight = 100.2 OF my_table WHERE id == 2;
SET ALL name = "Taro" OF my_table;
```
