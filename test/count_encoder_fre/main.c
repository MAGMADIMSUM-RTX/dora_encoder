#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <termios.h>
#include <sys/time.h>
#include <errno.h>
#include <stdint.h>

// CRC16 Modbus 计算函数
uint16_t crc16_modbus(const uint8_t *data, size_t length)
{
    uint16_t crc = 0xFFFF;
    for (size_t i = 0; i < length; i++)
    {
        crc ^= data[i];
        for (int j = 0; j < 8; j++)
        {
            if (crc & 0x0001)
            {
                crc = (crc >> 1) ^ 0xA001;
            }
            else
            {
                crc >>= 1;
            }
        }
    }
    return crc;
}

// 获取当前时间（毫秒精度）
double get_time_ms()
{
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (double)tv.tv_sec * 1000.0 + (double)tv.tv_usec / 1000.0;
}

int main()
{
    // 串口
    const char *port_name = "/dev/serial/by-id/usb-1a86_USB_Serial-if00-port0";
    int baud_rate = 1000000; // 1Mbps

    // 打开串口
    int fd = open(port_name, O_RDWR | O_NOCTTY | O_SYNC);
    if (fd < 0)
    {
        fprintf(stderr, "无法打开串口 %s: %s\n", port_name, strerror(errno));
        return 1;
    }

    // 配置串口参数
    struct termios tty;
    memset(&tty, 0, sizeof(tty));
    if (tcgetattr(fd, &tty) != 0)
    {
        fprintf(stderr, "获取串口配置失败: %s\n", strerror(errno));
        close(fd);
        return 1;
    }

    // 设置波特率
    cfsetospeed(&tty, B1000000);
    cfsetispeed(&tty, B1000000);

    // 8N1 (8位数据, 无奇偶校验, 1个停止位)
    tty.c_cflag = (tty.c_cflag & ~CSIZE) | CS8;
    tty.c_cflag &= ~(PARENB | PARODD); // 无奇偶校验
    tty.c_cflag &= ~CSTOPB;            // 1个停止位
    tty.c_cflag |= CREAD | CLOCAL;     // 开启接收，忽略调制解调器控制线

    // 设置为原始模式
    tty.c_iflag &= ~(IGNBRK | BRKINT | PARMRK | ISTRIP | INLCR | IGNCR | ICRNL | IXON);
    tty.c_lflag &= ~(ECHO | ECHONL | ICANON | ISIG | IEXTEN);
    tty.c_oflag &= ~OPOST;

    // 设置超时
    tty.c_cc[VMIN] = 0;  // 读取最小字符数
    tty.c_cc[VTIME] = 1; // 读取超时，1个100毫秒单位 = 100毫秒

    // 应用配置
    if (tcsetattr(fd, TCSANOW, &tty) != 0)
    {
        fprintf(stderr, "设置串口配置失败: %s\n", strerror(errno));
        close(fd);
        return 1;
    }

    // 主循环
    while (1)
    {
        // 记录发送时间
        double send_time = get_time_ms();

        // 准备发送数据
        uint8_t send_data[8] = {1, 0x03, 0x00, 0x42, 0x00, 0x02, 0x00, 0x00};
        uint16_t crc = crc16_modbus(send_data, 6);
        send_data[6] = crc & 0xFF;
        send_data[7] = (crc >> 8) & 0xFF;

        // 发送数据
        if (write(fd, send_data, sizeof(send_data)) != sizeof(send_data))
        {
            fprintf(stderr, "发送数据失败: %s\n", strerror(errno));
            continue;
        }

        // 读取响应
        uint8_t buf[10];
        ssize_t n = read(fd, buf, sizeof(buf));
        if (n < 0)
        {
            fprintf(stderr, "读取数据失败: %s\n", strerror(errno));
            continue;
        }
        double recv_time = get_time_ms();

        // 计算发送和接收的间隔
        double send_recv_interval = recv_time - send_time;
        printf("本次发送到接收的间隔: %.3f ms\n", send_recv_interval);
    }

    // 关闭串口 (实际上永远不会执行到这里，因为有无限循环)
    close(fd);
    return 0;
}
