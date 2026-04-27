package dev.mpclipboard.android.cli

import android.content.Context
import android.system.ErrnoException
import android.system.Os
import android.system.OsConstants
import android.system.StructPollfd
import android.os.Looper
import dev.mpclipboard.android.Connectivity
import dev.mpclipboard.android.MPClipboard
import dev.mpclipboard.android.Output
import java.io.BufferedReader
import java.io.FileDescriptor
import java.io.InputStreamReader

object Main {
    private const val RED = "\u001b[31m"
    private const val GREEN = "\u001b[32m"
    private const val YELLOW = "\u001b[33m"
    private const val NC = "\u001b[0m"
    private const val INFO =
        "\n\nThis is a demo of MPClipboard.\n" +
            "It reads lines from stdin and sends them to MPClipboard.\n" +
            "At the same time it polls MPClipboard and prints every received clip.\n\n"

    @JvmStatic
    fun main(args: Array<String>) {
        require(args.size == 3) {
            "Usage: app_process ... dev.mpclipboard.android.cli.Main <host> <token> <name>"
        }

        print("$GREEN$INFO$NC\n")

        val context = systemContext()
        check(MPClipboard.init(context)) { "MPClipboard.init() failed" }

        val mpclipboard = checkNotNull(MPClipboard.initialize(args[0], args[1], args[2])) {
            "MPClipboard.initialize() failed"
        }
        val mpclipboardFd = fileDescriptor(mpclipboard.getFd())
        val stdin = BufferedReader(InputStreamReader(System.`in`))

        while (true) {
            val fds = arrayOf(
                StructPollfd().apply {
                    fd = FileDescriptor.`in`
                    events = OsConstants.POLLIN.toShort()
                },
                StructPollfd().apply {
                    fd = mpclipboardFd
                    events = OsConstants.POLLIN.toShort()
                },
            )

            try {
                Os.poll(fds, -1)
            } catch (err: ErrnoException) {
                if (err.errno == OsConstants.EINTR) {
                    continue
                }
                throw err
            }

            if ((fds[0].revents.toInt() and OsConstants.POLLIN) != 0) {
                val line = stdin.readLine() ?: return
                mpclipboard.pushText(line)
            }

            if ((fds[1].revents.toInt() and OsConstants.POLLIN) != 0) {
                val output = mpclipboard.read() ?: continue
                printOutput(output)
            }
        }
    }

    private fun systemContext(): Context {
        if (Looper.myLooper() == null) {
            Looper.prepareMainLooper()
        }
        val activityThreadClass = Class.forName("android.app.ActivityThread")
        val activityThread = activityThreadClass.getMethod("systemMain").invoke(null)
        return activityThreadClass.getMethod("getSystemContext").invoke(activityThread) as Context
    }

    private fun fileDescriptor(fd: Int): FileDescriptor {
        val fileDescriptor = FileDescriptor()
        val setInt = FileDescriptor::class.java.getDeclaredMethod("setInt\$", Int::class.javaPrimitiveType)
        setInt.isAccessible = true
        setInt.invoke(fileDescriptor, fd)
        return fileDescriptor
    }

    private fun printConnectivity(connectivity: Connectivity) {
        when (connectivity) {
            Connectivity.Connecting -> println("${RED}connecting$NC")
            Connectivity.Connected -> println("${RED}connected$NC")
            Connectivity.Disconnected -> println("${RED}disconnected$NC")
        }
    }

    private fun printOutput(output: Output) {
        when (output) {
            is Output.ConnectivityChanged -> printConnectivity(output.connectivity)
            is Output.NewText -> println("${YELLOW}${output.text}$NC")
        }
    }
}
