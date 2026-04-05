package com.httparena.spring.boot;

import org.springframework.beans.factory.annotation.Qualifier;
import org.springframework.http.MediaType;
import org.springframework.jdbc.core.simple.JdbcClient;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RequestParam;
import org.springframework.web.bind.annotation.RestController;
import tools.jackson.core.type.TypeReference;
import tools.jackson.databind.json.JsonMapper;

import java.util.List;

@RestController
@RequestMapping("/async-db")
public class AsyncDatabaseQueryController {
    private final JdbcClient jdbcClient;
    private final JsonMapper jsonMapper;

    public AsyncDatabaseQueryController(@Qualifier("postgresql") JdbcClient jdbcClient, final JsonMapper jsonMapper) {
        this.jdbcClient = jdbcClient;
        this.jsonMapper = jsonMapper;
    }

    @GetMapping(produces = MediaType.APPLICATION_JSON_VALUE)
    public Items getItems(
            @RequestParam(defaultValue = "10") float min,
            @RequestParam(defaultValue = "50") float max)
    {
        try (var stream = jdbcClient.sql("""
                        SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count
                        FROM items
                        WHERE price BETWEEN ? AND ?
                        LIMIT 50""")
                .param(1, min)
                .param(2, max)
                .query(ItemRow.class)
                .stream())
        {
            List<Item> items = stream
                    .map(this::toItem)
                    .toList();
            return new Items(items, items.size());
        }
    }

    private Item toItem(ItemRow itemRow) {
        return new Item(
                itemRow.id(),
                itemRow.name(),
                itemRow.category(),
                itemRow.price(),
                itemRow.quantity(),
                itemRow.active(),
                jsonMapper.readValue(itemRow.tags(), new TypeReference<>() {}),
                new Rating(itemRow.ratingScore(), itemRow.ratingCount())
        );
    }

    record ItemRow(int id, String name, String category, float price, int quantity, boolean active, String tags, float ratingScore, int ratingCount) {
    }
}
