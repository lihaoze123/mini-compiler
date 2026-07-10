int f() {
    int res = 0;
    int i;
    for (i = 1; i <= 50; i++) {
        if (i % 2 == 1) {
            continue;
        }
        res = res + i;
    }
    return res;
}
