
version 1.1

import "local.wdl" as local alias Foo as Bar
import "https://example.com/remote.wdl" as remote alias Baz as Blorf

struct Example1 {
    Float f
    Map[String, Int] m
}

struct Example2 {
    String s
    Int? i
    Array[File?]+ a
    Example1 e
}

workflow Workflow1 {
    input {
        String s
        Int i = 0
        Example2? ex
    }

    Float f = i + 1.0
    Array[File] file_array = if defined(ex) then select_all(select_first([ex]).a) else []
    
    call local.foo
    call local.foo as bar {}
    call local.baz {
        input:
    }
    call remote.waldo {
        input:
            x = 1,
            y = false
    }

    if (1 > 2) {
        scatter (file in file_array) {
            call task1 {
                input:
                  file,
                  ex,
                  docker_image = "ubuntu"
            }
        }
    }

    output {
        Array[File]? f = task1.name_file
    }

    meta {
        description: "Test workflow"
        test: true
        size: 10
        numbers: [1, 2, 3]
        keywords: {
            a: 1.0,
            b: -1
        }
        x: null
    }
}

task Task1 {
    input {
        File file
        Example2? ex
        String docker_image
    }

    command <<<
    echo ~{file} \
      | cat
    >>>

    output {
        File name_file = stdout()
    }
    
    runtime {
        container: docker_image
    }

    meta {
        description: "write name to file"
    }
}
