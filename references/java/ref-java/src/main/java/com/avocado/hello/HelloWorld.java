package com.avocado.hello;

public class HelloWorld {
    public static void main(String[] args) {
        System.out.println("Hello from Avocado Linux!");
        System.out.println("Java version: " + System.getProperty("java.version"));
        System.out.println("Java vendor: " + System.getProperty("java.vendor"));
        System.out.println("OS: " + System.getProperty("os.name") + " " + System.getProperty("os.arch"));
    }
}
