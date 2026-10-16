#[struct]
make point {
    x: 3,
    y: 4,
}

write(point)
write(point.x)

point.x = 10
write(point.x)
write(point)