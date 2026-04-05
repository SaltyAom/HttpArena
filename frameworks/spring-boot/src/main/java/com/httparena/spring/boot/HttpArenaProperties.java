package com.httparena.spring.boot;

import org.springframework.boot.context.properties.ConfigurationProperties;

@ConfigurationProperties("httparena")
public record HttpArenaProperties(String postgresUrl) {
}
