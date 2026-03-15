package com.rustyiptv.ui

import androidx.compose.runtime.Composable
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import com.rustyiptv.bridge.CoreViewModel

@Composable
fun AppNavigation(viewModel: CoreViewModel) {
    val navController = rememberNavController()

    NavHost(navController = navController, startDestination = "home") {
        composable("home") {
            HomeScreen(
                viewModel = viewModel,
                onChannelSelected = { ch ->
                    navController.navigate("player/${ch.id}/${ch.name}")
                }
            )
        }
        composable("browse") {
            ChannelBrowserScreen(
                viewModel = viewModel,
                onChannelSelected = { ch ->
                    navController.navigate("player/${ch.id}/${ch.name}")
                },
                onChannelLongPress = { /* context menu - future */ }
            )
        }
        composable(
            route = "player/{channelId}/{channelName}",
            arguments = listOf(
                navArgument("channelId") { type = NavType.LongType },
                navArgument("channelName") { type = NavType.StringType }
            )
        ) { backStackEntry ->
            PlayerScreen(
                channelId = backStackEntry.arguments!!.getLong("channelId"),
                channelName = backStackEntry.arguments!!.getString("channelName") ?: "",
                viewModel = viewModel,
                onBack = { navController.popBackStack() }
            )
        }
        composable("search") {
            SearchScreen(
                viewModel = viewModel,
                onChannelSelected = { ch ->
                    navController.navigate("player/${ch.id}/${ch.name}")
                }
            )
        }
    }
}
