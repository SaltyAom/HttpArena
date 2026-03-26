<?php

class DB
{
    public static $db;
    public static $prepared;

    public static function init()
    {
        self::$db = new Sqlite3('/data/benchmark.db', SQLITE3_OPEN_READONLY);

        self::$prepared = self::$db->prepare('SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count
                            FROM items
                            WHERE price BETWEEN ? AND ?
                            LIMIT 50');
    }

    public static function query($min, $max)
    {
        self::$prepared->bindValue(1, $min, SQLITE3_FLOAT);
        self::$prepared->bindValue(2, $max, SQLITE3_FLOAT);

        $result = self::$prepared->execute();

        $data = [];
        while ($row = $result->fetchArray(SQLITE3_ASSOC)) {
            $data[] = [
                'id' => $row['id'],
                'name' => $row['name'],
                'category' => $row['category'],
                'price' => $row['price'],
                'quantity' => $row['quantity'],
                'active' => (bool) $row["active"],
                'tags' => json_decode($row["tags"], true),
                'rating' => [
                    "score" => $row["rating_score"],
                    "count" => $row["rating_count"]],
            ];
        }
        return json_encode(['items' => $data, 'count' => count($data)]);
    }
}
