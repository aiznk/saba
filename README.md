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
CREATE DATABASE mydb;
```

Create table (create CSV file).

```
CREATE TABLE mytable (
	id: INT PRIMARY_KEY AUTO_INCREMENT,
	weight: FLOAT,
	name: CHAR[100],
);
```

## DROP

Drop database (remove directory).

```
DROP DATABASE mydb;
```

Drop table (remove CSV file).

```
DROP TABLE mytable;
```

## USE

Select current using database.

```
USE mydb
```

## DESC

Show table info.

```
DESC mytable;
```

## ALTER (change table)

Add column into table.

```
ALTER TABLE mytable ADD COLUMN name CHAR[100];
```

Drop column from table.

```
ALTER TABLE mytable DROP COLUMN name;
```

Rename table name.

```
ALTER TABLE mytable REANME TO new_table;
```

Change column type.

```
ALTER TABLE mytable ALTER COLUMN id TYPE INT AUTO_INCREMENT;
```

## GET (get CSV records)

```
GET id, weight, name OF mytable;
GET id, weight, name OF mytable WHERE id == 2;
GET ALL id, weight, name OF mytable WHERE id < 5;
GET ALL * OF mytable WHERE id < 5 ORDER BY id;
```

## ADD (add CSV record)

```
ADD id = 1, weight = 60.3, name = "hige" OF mytable;
```

SQL like statement.

```
ADD OF mytable (id, weight, name) VALUES (1, 1.23, "hige"), (2, 2.23, "hoge");
ADD OF mytable VALUES (1, 1.23, "hige"), (2, 2.23, "hoge");
```

## DEL (delete CSV records)

```
DEL OF mytable WHERE id == 2;
DEL ALL OF mytable;
DEL ALL OF mytable WHERE id < 5;
```

## SET (update CSV records)

```
SET weight = 100.2 OF mytable WHERE id == 2;
SET ALL name = "Taro" OF mytable;
```

## Functions

### COUNT

```
GET ALL COUNT(*) OF mytable;
```

### SUM

```
GET ALL SUM(id) OF mytable;
```

### AVG

```
GET ALL AVG(id) OF mytable;
```

### MIN, MAX

```
GET ALL MIN(id) OF mytable;
GET ALL MAX(id) OF mytable;
```
