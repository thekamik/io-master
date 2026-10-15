// DO NOT USE: TO BE REWORKED (step 12 of docs/todo.md, review).
//
// Using this function in its present form is forbidden: it must
// not be called from anywhere until it has been reworked and
// approved by the reviewer (kk). Why:
//
// - It is a console function: it prints (println!) and reads the
//   confirmation from the terminal (stdin). CLAUDE.md allows no
//   console interface; questions to the user go through the web
//   UI, and core functions never print or read the terminal.
// - It asks for no authorization: whoever can type in the
//   terminal of the account removes the installation, with the
//   data of every user of the workstation. The removal is to be
//   moved to the web UI, after a login and a confirmation.
// - It returns an ExitCode (a result for the process) instead of
//   a result that the web UI can show.
//
// Open decisions for the rework: who may remove the installation
// on a workstation with several users, and what the program says
// about a damaged installation when it cannot be removed from the
// web UI.

/// removes the installation after the user typed
/// the confirmation.

fn uninstall(data_dir: &Path) -> ExitCode {
    println!("This removes the installation of IO Master in {}.", data_dir.display());
    println!("Its data cannot be opened afterwards.");
    println!("Type '{UNINSTALL_CONFIRMATION}' to continue:");

    let mut answer = String::new();

    if std::io::stdin().lock().read_line(&mut answer).is_err() || answer.trim() != UNINSTALL_CONFIRMATION {
        println!("Nothing was removed.");
        return ExitCode::FAILURE;
    }

    if let Err(error) = lifecycle::uninstall(data_dir) {
        eprintln!("Error: {error}");
        return ExitCode::FAILURE;
    }

    println!("The installation was removed.");

    ExitCode::SUCCESS
}