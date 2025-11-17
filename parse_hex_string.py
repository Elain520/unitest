def parse_hexstring(s: str):
    length = 0
    byte_data = []
    for num in s.split(' '):
        if s.startswith("0x"):
            num = num[2:]
        while len(num) > 0:
            byte_num = num[-2:]
            byte_data.append(int(byte_num, 16))
            length += 1
            num = num[0:-2]
    return length, byte_data


if __name__ == '__main__':
    print(parse_hexstring("9abcdef0 12345678"))
    print(parse_hexstring("fa aa 55 33"))
    print(parse_hexstring("0x123456789"))
