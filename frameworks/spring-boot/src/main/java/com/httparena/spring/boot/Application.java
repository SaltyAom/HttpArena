package com.httparena.spring.boot;

import org.springframework.beans.factory.annotation.Qualifier;
import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.context.annotation.Bean;
import org.springframework.jdbc.core.simple.JdbcClient;
import org.sqlite.SQLiteDataSource;

import javax.sql.DataSource;

@SpringBootApplication
public class Application {

	public static void main(String[] args) {
		SpringApplication.run(Application.class, args);
	}

	@Qualifier("sqlite")
	@Bean
	public DataSource sqliteDataSource() {
		SQLiteDataSource sqLiteDataSource = new SQLiteDataSource();
		sqLiteDataSource.setUrl("jdbc:sqlite:/data/benchmark.db");
		sqLiteDataSource.setReadOnly(true);
		return sqLiteDataSource;
	}

	@Bean
	@Qualifier("sqlite")
	public JdbcClient sqliteJdbcClient(@Qualifier("sqlite") DataSource dataSource) {
		return JdbcClient.create(dataSource);
	}
}
