# AuthenticationServer — player login and account management.

add_executable(AuthenticationServer
    ../../authentication/frontend_connection.cpp
    ../../authentication/launcher.cpp
    ../../authentication/logon_connection.cpp
    ../../authentication/logon_queue.cpp
    ../../authentication/service_main.cpp
)

target_precompile_headers(AuthenticationServer REUSE_FROM unified_kernel)

target_link_libraries(AuthenticationServer PRIVATE
    unified_kernel
)
