cmake_minimum_required(VERSION 3.10)
project(HarryDebugSwitch C)

add_application(HarryDebugSwitch
    debug_switch.c
    debug_switch.r
    TYPE "APPL"
    CREATOR "HDbS"
    )
set_target_properties(HarryDebugSwitch PROPERTIES
    COMPILE_OPTIONS "-ffunction-sections;-Os;-Wall"
    LINK_FLAGS "-Wl,-gc-sections")
