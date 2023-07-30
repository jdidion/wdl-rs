
version 1.1

workflow Test {
    # assignment

    Boolean t = true
    Boolean f = false

    Int i1 = 100
    Int i2 = -1
    Int i3 = +1
    Int i4 = 0xFFFF
    Int i5 = 0777

    Float f1 = 1e5
    Float f2 = .123
    Float f3 = .1e2
    Float f4 = 1.2
    Float f5 = 1.2E5
    Float f6 = +1.0
    Float f7 = -1.0

    Array[Int] a0 = []
    Array[Int] a1 = [1, 2, 3]

    Map[Int, Boolean] m0 = {}
    Map[Int, Boolean] m1 = {
        1: true,
        2: false
    }

    Pair[Int, Float] p1 = (-1, 2.0)

    Object obj0 = object {}
    Object obj1 = object {
        a: true,
        b: 1,
        c: -1.0
    }

    Foo foo = Foo {
        a: true,
        b: 1,
        c: -1.0
    }
    
    # binary operations

    Int bin1 = 1 + 2 + 3
    Int bin2 = 1 + 2 * 3
    Int bin3 = -(1 + 2) * 3
    Float bin4 = bin2 / bin3

    Boolean bool1 = true && false 
    Boolean bool2 = 1 < 2
    Boolean bool3 = 1 >= 2 || 3 == 4
    Boolean bool4 = !(true || false)

    # other

    Array[Int?] a2 = [1, None]
    Int? i = a2[0]
    Int j = select_first(a2)
    Int k = select_first([a2[0]])
    Boolean foo_a = foo.a
    Int x = if 1 > 2 then 0 else 1
}
