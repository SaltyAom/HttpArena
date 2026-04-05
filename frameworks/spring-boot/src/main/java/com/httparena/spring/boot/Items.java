package com.httparena.spring.boot;

import java.util.List;

public record Items(List<Item> items, long count) {
}
