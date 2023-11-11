cmake_minimum_required(VERSION 3.22)
project(dcfs2)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_MODULE_PATH "${CMAKE_MODULE_PATH}" "${CMAKE_SOURCE_DIR}/cmake")

find_package(Protobuf REQUIRED)
file(GLOB ProtoFiles "${CMAKE_CURRENT_SOURCE_DIR}/*.proto")
include_directories(${Protobuf_INCLUDE_DIRS})
include_directories(${CMAKE_CURRENT_BINARY_DIR})
protobuf_generate_cpp(PROTO_SRCS PROTO_HDRS ${ProtoFiles})

find_package(FUSE3 REQUIRED)
find_package(OpenSSL REQUIRED)

add_executable(dcfs2 main.cpp ${PROTO_SRCS} ${PROTO_HDRS})
target_link_libraries(dcfs2 ${Protobuf_LIBRARIES} ${FUSE3_LIBRARIES} OpenSSL::SSL)