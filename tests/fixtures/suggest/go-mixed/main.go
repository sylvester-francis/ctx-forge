package main

import (
    "fmt"
    "github.com/gin-gonic/gin"
    "golang.org/x/sync/errgroup"
)

func main() {
    _ = fmt.Sprintf
    _ = gin.New
    _ = errgroup.WithContext
}
