int main() {
    int x = 10;
    {
        x = x + 1;
        int x = 10;
    }
    return x;
}
