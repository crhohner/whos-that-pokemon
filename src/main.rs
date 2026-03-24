

use serde::{Serialize, Deserialize};
use crossterm::{event::{self, Event, KeyCode, KeyEvent, KeyEventKind}};
use ratatui::{
    DefaultTerminal, Frame, buffer::Buffer, layout::{Constraint, Direction, Layout, Rect}, style::Modifier, symbols::border, text::Line, widgets::{Block, Paragraph, Widget}
};
use ratatui::style::{Color, Style};
use rand::{RngExt, rngs::ThreadRng};
use tui_logger::*;
use log::debug;
use color_eyre::{
    eyre::{bail, WrapErr, eyre},
    Result,
};




const API: &str = "https://pokeapi.co/api/v2/";

const ASCII_TEST: &str = "                                                                                                    
                                                                                                    
                                                                                                                                   
                                                                                                    
                                           ++=+##@  #@@@@                                           
                                    ++*##%#*-:-=++#*::-=#@@                                         
                                   +-::-=======++===--===%@                                         
                                  #+-:-=================++*#%                                       
                                  @%+======+*======#========+**%                                    
                                   @#+=====+++=================*@                                   
                                    @#========*%@@@%*==========*@                                   
                                    %*========================*%%                                   
                                   %*=======================+++*#                                   
                                 @@*=========================+++#@                                  
                                #*+==========================++++*#                                 
                               %**+++=======================++++++*#@                               
                               @#+++++++++++==========+++++++++++++*@                               
                                @#++++++++++++====+++=+=+++++++++++#@                               
                                 @@#++#@@##++++++++++++++#@@%*++#@@                                 
                                   @@@@  %##%+++++++++%%%%  %@@@%                                   
                                            @@@@@@@@@@@                                             
                                                                                                    
                                                                                                    
                                                                                                    ";


#[derive(Serialize, Deserialize, Debug, Default)]
struct PokedexEntry {
    name: String,
    url: String
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Pokedex {
    count: i32, //remove if conclusively cant be used to generate indices
    results: Vec<PokedexEntry>
}

#[derive(Deserialize, Debug, Default)]
struct PokemonSpecies {
    varieties: Vec<PokemonVariety>
}

#[derive(Deserialize, Debug, Default)]
struct PokemonVariety {
    is_default: bool,
    pokemon: PokedexEntry
}

#[derive(Deserialize, Debug, Default)]
struct Pokemon {
    name: String,
    //todo - grab the default information instead
    height: i32,
    types: Vec<PokeTypeSlot>,
    sprites: PokeSpriteURL
}

#[derive(Deserialize, Debug, Default)]
struct PokeSpriteURL {
    front_default: String
}

#[derive(Deserialize, Debug)]
struct PokeTypeSlot {
    r#type: PokeType
}

#[derive(Deserialize, Debug)]
struct PokeType {
    name: String
}


impl Pokedex {
    pub fn init(&mut self) -> Result<()> { //ignore this slightly strange pattern pls
   
        let pokedex_url = format!("{}{}", API, "pokemon-species?limit=100000&offset=0");

        let json = reqwest::blocking::get(pokedex_url)?
            .error_for_status()?     
            .json::<Pokedex>()?;
       
        *self = json;
        Ok(())
    }

    pub fn get_info(&mut self, poke_index: usize) -> Result<Pokemon> {

        let pokemon_url = &self.results[poke_index].url;

        let species_json = reqwest::blocking::get(pokemon_url)?
            .error_for_status()?     
            .json::<PokemonSpecies>()?;
    
        let species = species_json
            .varieties
            .iter()
            .find(|var| var.is_default)
            .ok_or_else(|| eyre!("no default pokemon found for species {}", &self.results[poke_index].name))?;
    
        let pokemon = reqwest::blocking::get(&species.pokemon.url)?
            .error_for_status()?     
            .json::<Pokemon>()?;
    
        Ok(pokemon)
    }


}

#[derive(Debug, Default)]
pub struct App {
    pokedex: Pokedex,
    exit: bool,
    rng: ThreadRng,
    pokemon: Pokemon
}


impl App {

    pub fn init(&mut self) -> color_eyre::Result<()> {
        self.pokedex.init()?;
        self.rng = rand::rng();
        self.next_pokemon()?;

        Ok(())
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {

        self.init()?;

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> color_eyre::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            // ratatui tutorial comments ^^^
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)?;
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Char('q') => {self.exit = true; Ok(())}
            KeyCode::Char('n') => {
                debug!("{:?}", self.pokedex.count);
                self.next_pokemon()?;
                Ok(())
            }
            _ => {Ok(())}
        }
    }

    fn next_pokemon(&mut self) -> color_eyre::Result<()>{
        let n = self.rng.random_range(0..self.pokedex.count as usize);
        self.pokemon = self.pokedex.get_info(n)?;
        Ok(())
    }

}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {

        let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

        let title = Line::from("WHO'S THAT POKEMON?").style(Style::default().add_modifier(Modifier::REVERSED));
        let instructions = Line::from(vec![
            " next ".into(),
            "<n> ".into(),
            " quit ".into(),
            "<q> ".into(),
        ]);

        //this is the border!! applied in the paragraph construction
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        // let poke_text = Text::from(vec![Line::from(vec![
        //     "pokemon: ".into(),
        //     self.pokemon.name.clone().yellow(),
        // ])]);

        // Paragraph::new(poke_text)
        //     .centered()
        //     .block(block)
        //     .render(layout[1], buf);

        Paragraph::new(ASCII_TEST).centered().block(block).render(layout[1], buf);

        TuiLoggerWidget::default()
            .block(Block::bordered().title("logs"))
            .output_separator('|')
            .output_level(Some(TuiLoggerLevelOutput::Long))
            .style(Style::default().fg(Color::White))
            .render(layout[0], buf);
    
       
    }
}

fn main() -> color_eyre::Result<()> {

    tui_logger::init_logger(LevelFilter::Debug).unwrap();
    tui_logger::set_default_level(LevelFilter::Debug);

    color_eyre::install()?;

    ratatui::run(|terminal| App::default().run(terminal))

}


//TODO 
/*
- correct eyre setup?
- basic ascii image display (scale?)
- game/input/streaks
- cleanup, improve errors
*/
