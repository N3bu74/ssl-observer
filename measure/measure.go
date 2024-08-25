package main

import (
	"crypto/tls"
	"database/sql"
	"fmt"
	"github.com/schollz/progressbar/v3"
	"io"
	"log"
	"net/http"
	"os"
	"reflect"
	"strings"
	"sync"
	"time"

	_ "github.com/go-sql-driver/mysql"
)

const (
	dbHost       = "192.168.25.129"
	dbUser       = "root"
	dbPassword   = "root"
	dbName       = "ssl_data"
	delay        = 5 * time.Second // 假设delay为10秒，
	testTimes    = 10              // 重复测试次数，根据需要调整
	testUrl      = "https://192.168.25.129/test.html"
	numRequests  = 1000
	fileToWrite  = ".\\result.txt"
	readAndWrite = 2 // 测试读写的总数
)

// timeToDelay 延时一段时间，并显示进度条
func timeToDelay() {
	// 初始化一个进度条，总长度为1，持续时间为delay
	bar := progressbar.NewOptions64(
		int64(delay/time.Millisecond), // 将时间转换为毫秒，以便于计算
		progressbar.OptionSetDescription("Waiting..."),
		progressbar.OptionSetTheme(progressbar.Theme{
			Saucer:        "=",
			SaucerHead:    ">",
			SaucerPadding: " ",
			BarStart:      "[",
			BarEnd:        "]",
		}),
		progressbar.OptionShowCount(),
		//progressbar.OptionOnCompletion(func() {
		//	fmt.Fprintln(os.Stderr, "Delay completed.")
		//}),
	)

	// 计算每次更新的间隔（例如，每100毫秒更新一次）
	updateInterval := 100 * time.Millisecond
	totalUpdates := delay / updateInterval

	// 更新进度条
	ticker := time.NewTicker(updateInterval)
	defer ticker.Stop()

	for i := 0; i < int(totalUpdates); i++ {
		select {
		case <-ticker.C:
			bar.Add64(updateInterval.Milliseconds())
		}
	}

	// 确保进度条在结束时完全填充
	remaining := delay % updateInterval
	if remaining > 0 {
		time.Sleep(remaining)
		bar.Add64(remaining.Milliseconds())
	}

	bar.Finish()
}

func main() {
	var numbers []int
	var successCounts []int
	var durations []float64
	db, err := sql.Open("mysql", fmt.Sprintf("%s:%s@tcp(%s)/%s", dbUser, dbPassword, dbHost, dbName))
	if err != nil {
		log.Fatalf("Failed to open database connection: %v", err)
		return
	}
	defer func(db *sql.DB) {
		err := db.Close()
		if err != nil {
			log.Println("Error closing database connection:", err)
		}
	}(db)

	for i := 0; i < testTimes; i++ {
		// 1. 进行并发请求测试
		totalSuccess, duration, err := SimulateConcurrentRequests(testUrl, numRequests, true)
		if err != nil {
			log.Printf("Error during test %d: %v", i+1, err)
			continue
		}
		totalSuccess = totalSuccess * readAndWrite

		// 2. 等待一段时间
		timeToDelay()
		//time.Sleep(delay)

		// 3. 检验并发测试效果
		number := measure(db)
		numbers = append(numbers, number)
		successCounts = append(successCounts, totalSuccess)
		durations = append(durations, duration.Seconds())

		// 4. 打印当前结果
		fmt.Printf("Test %3d: successCount = %6d, result = %6d,duration = %v\n", i+1, totalSuccess, number, duration)
	}

	// 5. 输出结果数组写入到 fileToWrite 文件中
	writeResultsToFile(fileToWrite, numbers, successCounts, durations)
}

// measure 对程序进行测试
func measure(db *sql.DB) int {
	var number int
	tx, err := db.Begin()
	if err != nil {
		log.Fatalf("Begin transaction error: %v", err)
	}

	// 使用事务执行查询
	row := tx.QueryRow("SELECT COUNT(id) as number FROM ssl_data")
	err = row.Scan(&number)
	if err != nil {
		log.Fatalf("Query row error: %v", err)
	}

	// 清空表数据
	_, err = tx.Exec("TRUNCATE TABLE ssl_data")
	if err != nil {
		log.Fatalf("Truncate table error: %v", err)
	}

	if err = tx.Commit(); err != nil {
		log.Fatal(err)
	}

	return number
}

// printNumbersWithCommas 返回一个字符串，表示数字数组，元素之间用逗号隔开
func printNumbersWithCommas(numbers interface{}) string {
	v := reflect.ValueOf(numbers)
	var builder strings.Builder
	builder.WriteString("[") // 开始的方括号

	// 使用 for 循环遍历数组
	for i := 0; i < v.Len(); i++ {
		// 使用Sprintf将数字转换为字符串并添加到builder
		builder.WriteString(fmt.Sprintf("%v", v.Index(i).Interface()))
		if i < v.Len()-1 { // 如果不是最后一个元素，添加逗号
			builder.WriteString(", ")
		}
	}

	builder.WriteString("]") // 结束的方括号
	return builder.String()
}

// writeResultsToFile 将结果输入到文件中。
func writeResultsToFile(filePath string, numbers, successCounts []int, durations []float64) {
	numbersString := printNumbersWithCommas(numbers)
	successString := printNumbersWithCommas(successCounts)
	durationsString := printNumbersWithCommas(durations)
	fmt.Println(numbersString)
	fmt.Println(successString)
	fmt.Println(durationsString)

	file, err := os.OpenFile(filePath, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, 0666)
	if err != nil {
		log.Fatalf("Error opening file: %v", err)
	}
	defer file.Close()

	_, err = file.WriteString(fmt.Sprintf("%s\n%s\n%s\n",
		numbersString,
		successString,
		durationsString))
	if err != nil {
		log.Fatalf("Error writing to file: %v", err)
	}
}

// SimulateConcurrentRequests 执行并发HTTP GET请求并统计成功次数。
func SimulateConcurrentRequests(url string, numRequests int, insecureSkipTLS bool) (int, time.Duration, error) {
	startTime := time.Now()

	tr := &http.Transport{
		TLSClientConfig: &tls.Config{InsecureSkipVerify: insecureSkipTLS},
	}
	client := &http.Client{
		Transport: tr,
	}

	var wg sync.WaitGroup
	successCount := make(chan int, numRequests)

	// 创建一个进度条
	pb := progressbar.NewOptions(numRequests,
		progressbar.OptionSetDescription("Executing Requests"),
		progressbar.OptionShowCount(),
		//progressbar.OptionOnCompletion(func() { fmt.Fprintln(os.Stderr, "\rThe concurrent request is over.") }),
		progressbar.OptionSpinnerType(14),
		progressbar.OptionSetWidth(10),
		progressbar.OptionSetPredictTime(true),
		progressbar.OptionClearOnFinish(),
	)

	for i := 0; i < numRequests; i++ {
		wg.Add(1)
		go func(idx int) {
			defer wg.Done()
			req, err := http.NewRequest("GET", url, nil)
			if err != nil {
				log.Printf("Request creation failed for request %d: %v", i, err)
				return
			}
			req.Header.Add("Accept-Encoding", "gzip")

			resp, err := client.Do(req)
			if err != nil {
				log.Printf("Request failed for request %d: %v", i, err)
				return
			}
			defer func(Body io.ReadCloser) {
				err := Body.Close()
				if err != nil {

				}
			}(resp.Body)

			if resp.StatusCode != http.StatusOK {
				log.Printf("Request returned non-200 status for request %d: %s", i, resp.Status)
				return
			}

			successCount <- 1
			pb.Add(1)
		}(i)
	}

	go func() {
		wg.Wait()
		close(successCount)
	}()

	totalSuccess := 0
	for count := range successCount {
		totalSuccess += count
	}

	endTime := time.Now()
	duration := endTime.Sub(startTime)

	return totalSuccess, duration, nil
}
